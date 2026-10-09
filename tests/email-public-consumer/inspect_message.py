"""Independent stdlib RFC parsing of synthetic output from the public Rust consumer."""
import email.policy
import hashlib
import json
import pathlib
import sys
from email.parser import BytesParser
from email.utils import parsedate_to_datetime

x = json.loads(pathlib.Path(sys.argv[1]).read_text())
m = BytesParser(policy=email.policy.default).parsebytes(x["wire"].encode())
assert not m.defects, m.defects
assert str(m["Subject"]) == "Zażółć gęślą"
assert str(m["From"]) == x["from"] == "sender@example.invalid"
assert str(m["To"]) == x["to"] == "Exact+Tag@example.invalid"
assert m["Message-ID"] == x["id"] == "<" + hashlib.sha256(b"work-17").hexdigest() + "@host.example.invalid>"
assert parsedate_to_datetime(m["Date"]).timestamp() == 1800000000
assert m.get_content_type() == "text/plain"
assert m["MIME-Version"] == "1.0"
assert m.get_content_charset() == "utf-8"
assert m.get_content().replace("\r\n", "\n") == "Treść\n\n.\nDalszy wiersz\t0"
assert not m["Bcc"] and not m["Cc"]
print("Independent public email consumer and Python RFC/MIME decoding passed; no SMTP transport qualification")
