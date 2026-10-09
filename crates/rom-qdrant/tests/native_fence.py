"""Consume Rust-prepared writes; exercise the actual private persistent TLS fixture."""
import json
import sys
import uuid
import copy
import concurrent.futures
import threading
sys.path.insert(0, str(__import__("pathlib").Path(__file__).resolve().parents[3] / "tests/projection-probes"))
from qdrant_fixture import call

raw = sys.stdin.buffer.read(1048577)
assert len(raw) <= 1048576
requests = json.loads(raw)
name = "rom_extras_rust_identity_" + uuid.uuid4().hex
call("PUT", "/collections/" + name, {"vectors": {"embedding": {"size": 3, "distance": "Dot"}}})
identity = requests[0]["points"][0]["id"]

def send(body):
    result = call("PUT", "/collections/" + name + "/points?wait=true&ordering=strong", body)
    assert result["result"]["status"] == "completed"

def stored():
    return call("GET", "/collections/" + name + "/points/" + identity, credential="read")["result"]

send(requests[0])
assert stored()["payload"] == requests[0]["points"][0]["payload"]
# Force a digest-ID collision in the actual request, leaving its original-key condition unchanged.
requests[1]["points"][0]["id"] = identity
send(requests[1])
assert stored()["payload"] == requests[0]["points"][0]["payload"]
send(requests[4])
assert stored()["payload"] == requests[0]["points"][0]["payload"]
send(requests[2])
assert stored()["payload"] == requests[2]["points"][0]["payload"]
send(requests[0])
assert stored()["payload"] == requests[2]["points"][0]["payload"]
send(requests[3])
assert stored()["payload"] == requests[3]["points"][0]["payload"]
assert stored()["vector"] == {}
send(requests[2])
assert stored()["payload"] == requests[3]["points"][0]["payload"]
# Both requests observe an initially missing ID; the first accepted original key must win.
for _ in range(8):
    identity = str(uuid.uuid4())
    pair = [copy.deepcopy(requests[i]) for i in [0, 1]]
    for body in pair:
        body["points"][0]["id"] = identity
    barrier = threading.Barrier(2)
    def race(body):
        barrier.wait(timeout=5)
        send(body)
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
        list(pool.map(race, pair))
    winner = stored()["payload"]
    assert winner in [body["points"][0]["payload"] for body in pair]
    for body in pair:
        send(body)
    assert stored()["payload"] == winner
print(json.dumps({"passed": True, "collection": name, "requests": 39,
                  "concurrent_missing_collision_rounds": 8,
                  "scope": "actual generated Rust wire; missing insertion, original-key collision, equal mismatch, stale replay, u64 max tombstone"}))
