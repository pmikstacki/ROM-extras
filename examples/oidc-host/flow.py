"""Controlled qualification host; not a production session implementation.

No redirects occur automatically. Only an explicit HTTPS origin is contacted.
Credentials, cookies and tokens must remain in private host files.
"""
import hashlib
import hmac
import http.client
import ipaddress
import json
import secrets
import ssl
import socket
import threading
import time
from base64 import urlsafe_b64encode
from html.parser import HTMLParser
from urllib.parse import parse_qs, urlencode, urlsplit


class Rejected(Exception):
    def __init__(self):
        super().__init__('OIDC host request rejected')


class LoginState:
    def __init__(self, state, nonce, verifier, expires):
        self.state, self.nonce, self.verifier = state, nonce, verifier
        self.expires, self.used = expires, False

    def consume(self, location, callback, now):
        parsed = urlsplit(location)
        if self.used or now >= self.expires or parsed.fragment:
            raise Rejected()
        if parsed._replace(query='').geturl() != callback:
            raise Rejected()
        values = parse_qs(parsed.query, keep_blank_values=True, max_num_fields=8)
        if set(values) - {'state', 'code', 'session_state', 'iss'}:
            raise Rejected()
        if any(len(value) != 1 for value in values.values()):
            raise Rejected()
        state, code = values.get('state', [''])[0], values.get('code', [''])[0]
        if not hmac.compare_digest(state, self.state) or not code or len(code) > 4096:
            raise Rejected()
        self.used = True  # consume before exchange; retry requires a fresh login
        return code


class Host:
    def __init__(self, origin, ca_file, initialize=True):
        self.origin = origin
        self.parsed = urlsplit(origin)
        self.approve(origin)
        try:
            if not ipaddress.ip_address(self.parsed.hostname).is_loopback or not 1 <= (self.parsed.port or 443) <= 65535:
                raise Rejected()
        except ValueError:
            raise Rejected() from None
        if self.parsed.path or self.parsed.query:
            raise Rejected()
        self.context = ssl.create_default_context(cafile=ca_file) if initialize else None
        self.cookies = {}
        self.deadline = time.monotonic() + 60
        self.remaining = 24

    def approve(self, url):
        if len(url) > 8192 or any(ord(c) < 33 or c == '\\' for c in url):
            raise Rejected()
        p = urlsplit(url)
        if p.scheme != 'https' or p.netloc != self.parsed.netloc or p.username or p.password or p.fragment:
            raise Rejected()
        return (p.path or '/') + ('?' + p.query if p.query else '')

    def request(self, url, fields=None, bearer=None, limit=65536, document=None, method=None):
        path = self.approve(url)
        left = self.deadline - time.monotonic()
        if left <= 0 or self.remaining <= 0 or limit > 262144:
            raise Rejected()
        self.remaining -= 1
        headers = {'Accept': 'application/json, text/html'}
        body = None
        if fields is not None:
            body = urlencode(fields).encode()
            headers['Content-Type'] = 'application/x-www-form-urlencoded'
        if document is not None:
            if fields is not None:
                raise Rejected()
            body = json.dumps(document).encode()
            if len(body) > 65536:
                raise Rejected()
            headers['Content-Type'] = 'application/json'
        if bearer:
            headers['Authorization'] = 'Bearer ' + bearer
        if self.cookies:
            headers['Cookie'] = '; '.join(k+'='+v for k,v in self.cookies.items())
        selected_method = method or ('POST' if body is not None else 'GET')
        if selected_method not in ('GET', 'POST', 'PUT'):
            raise Rejected()
        connection = http.client.HTTPSConnection(self.parsed.hostname, self.parsed.port or 443, context=self.context, timeout=min(5, left))
        expired = threading.Event()
        held_socket = [None]
        def interrupt():
            expired.set()
            active = held_socket[0] or connection.sock
            if active is not None:
                try:
                    active.shutdown(socket.SHUT_RDWR)
                except OSError:
                    pass
        timer = threading.Timer(max(0, self.deadline-time.monotonic()), interrupt)
        timer.daemon = True
        timer.start()
        try:
            connection.request(selected_method, path, body, headers)
            # Retain the socket even when getresponse detaches a Connection: close stream.
            held_socket[0] = connection.sock
            if expired.is_set():
                raise Rejected()
            response = connection.getresponse()
            length = response.getheader('Content-Length')
            if length and (not length.isdecimal() or int(length) > limit):
                raise Rejected()
            data = bytearray()
            while True:
                left = self.deadline - time.monotonic()
                if left <= 0:
                    raise Rejected()
                if connection.sock:
                    connection.sock.settimeout(min(5, left))
                chunk = response.read1(min(8192, limit+1-len(data)))
                if not chunk:
                    break
                data.extend(chunk)
                if len(data) > limit:
                    raise Rejected()
            if expired.is_set() or time.monotonic() >= self.deadline:
                raise Rejected()
            for name, value in response.getheaders():
                if name.lower() == 'set-cookie':
                    pair = value.split(';', 1)[0].split('=', 1)
                    if len(pair) != 2 or len(pair[0]) > 128 or len(pair[1]) > 8192 or len(self.cookies) >= 16:
                        raise Rejected()
                    self.cookies[pair[0]] = pair[1]
            return response.status, response.getheader('Location'), bytes(data)
        except (OSError, http.client.HTTPException, ValueError):
            raise Rejected() from None
        finally:
            timer.cancel()
            timer.join()
            connection.close()

    def document(self, url):
        status, location, raw = self.request(url)
        if status != 200 or location:
            raise Rejected()
        try:
            return json.loads(raw), raw
        except (ValueError, UnicodeError):
            raise Rejected() from None


class LoginForm(HTMLParser):
    def __init__(self):
        super().__init__()
        self.actions = []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'form' and attrs.get('id') == 'kc-form-login':
            self.actions.append(attrs.get('action'))


def login(host, issuer, callback, credentials, wrong_pkce=False):
    discovery, _ = host.document(issuer+'/.well-known/openid-configuration')
    if discovery.get('issuer') != issuer:
        raise Rejected()
    endpoints = {key: issuer+'/protocol/openid-connect/'+suffix for key, suffix in [('authorization_endpoint','auth'),('token_endpoint','token'),('jwks_uri','certs')]}
    if any(discovery.get(k) != value for k, value in endpoints.items()):
        raise Rejected()
    state = LoginState(secrets.token_urlsafe(32), secrets.token_urlsafe(32), secrets.token_urlsafe(48), time.monotonic()+45)
    challenge = urlsafe_b64encode(hashlib.sha256(state.verifier.encode()).digest()).decode().rstrip('=')
    query = {'client_id':'rom-web','response_type':'code','scope':'openid','redirect_uri':callback,'state':state.state,'nonce':state.nonce,'code_challenge':challenge,'code_challenge_method':'S256','prompt':'login'}
    status, location, raw = host.request(endpoints['authorization_endpoint']+'?'+urlencode(query), limit=262144)
    if status != 200 or location:
        raise Rejected()
    form = LoginForm()
    form.feed(raw.decode())
    if len(form.actions) != 1 or not form.actions[0]:
        raise Rejected()
    status, location, _ = host.request(form.actions[0], {'username':credentials['user_username'],'password':credentials['user_password'],'credentialId':''}, limit=262144)
    if status not in (302,303) or not location:
        raise Rejected()
    callback_issuer = parse_qs(urlsplit(location).query).get('iss')
    if callback_issuer != [issuer]:
        raise Rejected()
    code = state.consume(location, callback, time.monotonic())
    fields = {'grant_type':'authorization_code','client_id':'rom-web','redirect_uri':callback,'code':code,'code_verifier':secrets.token_urlsafe(48) if wrong_pkce else state.verifier}
    status, _, raw = host.request(endpoints['token_endpoint'], fields)
    if wrong_pkce:
        if status != 400:
            raise Rejected()
        return {'wrong_pkce_rejected':True}
    if status != 200:
        raise Rejected()
    tokens = json.loads(raw)
    replay, _, _ = host.request(endpoints['token_endpoint'], fields)
    if replay != 400:
        raise Rejected()
    _, jwks = host.document(endpoints['jwks_uri'])
    return {'issuer':issuer,'client_id':'rom-web','nonce':state.nonce,'id_token':tokens['id_token'],'access_token':tokens['access_token'],'authorization_code':code,'jwks':jwks.decode(),'now':int(time.time()),'code_replay_rejected':True}
