"""Bounded Unix ownership for qualification process groups; no arbitrary PID cleanup."""
import contextlib
import os
import signal
import subprocess
import time

def install_interrupt_handlers():
    """Translate INT/TERM into unwinding so fixture finally blocks can restore state."""
    def interrupt(_number, _frame):
        raise KeyboardInterrupt('qualification interrupted')
    signal.signal(signal.SIGINT, interrupt)
    signal.signal(signal.SIGTERM, interrupt)

@contextlib.contextmanager
def defer_interrupts():
    """Defer Python INT/TERM handlers during bounded cleanup, including worker delivery."""
    previous = {number: signal.getsignal(number) for number in [signal.SIGINT, signal.SIGTERM]}
    pending = None
    def remember(number, frame):
        nonlocal pending
        if pending is None:
            pending = (number, frame)
    for number in previous:
        signal.signal(number, remember)
    try:
        yield
    finally:
        for number, handler in previous.items():
            signal.signal(number, handler)
        if pending is not None:
            number, frame = pending
            handler = previous[number]
            if callable(handler):
                handler(number, frame)
            elif handler != signal.SIG_IGN:
                signal.raise_signal(number)

def close_owned_group(child):
    """Close only the group created by wait_owned, including remaining descendants."""
    with defer_interrupts():
        try:
            os.killpg(child.pid, signal.SIGTERM)
        except ProcessLookupError:
            child.wait(timeout=1)
            return
        deadline = time.monotonic() + 2
        while time.monotonic() < deadline:
            child.poll()
            try:
                os.killpg(child.pid, 0)
            except ProcessLookupError:
                child.wait(timeout=1)
                return
            time.sleep(.02)
        try:
            os.killpg(child.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        child.wait(timeout=1)

def wait_owned(command, *, timeout, **kwargs):
    """Wait with a deadline; close the newly owned group on success or any exception."""
    child = subprocess.Popen(command, start_new_session=True, **kwargs)
    try:
        return child.wait(timeout=timeout)
    finally:
        close_owned_group(child)
