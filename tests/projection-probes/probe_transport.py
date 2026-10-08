"""Transport policy shared by the isolated real-service probes."""
import urllib.request


class RejectRedirects(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, response, code, message, headers, target):
        return None


def client(context):
    return urllib.request.build_opener(urllib.request.ProxyHandler({}), RejectRedirects(),
                                      urllib.request.HTTPSHandler(context=context))
