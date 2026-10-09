"""Actual detached process/group interruption; no service acceptance claims."""
import os, pathlib, signal, subprocess, sys, tempfile, threading, time, unittest
root=pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0,str(root/'scripts'))
from lib.fixture_processes import install_interrupt_handlers, wait_owned, defer_interrupts
class OwnedProcessTests(unittest.TestCase):
    def test_sigint_and_sigterm_close_owned_child_and_grandchild(self):
        for sig in [signal.SIGINT,signal.SIGTERM]:
            with self.subTest(signal=sig):
                directory=pathlib.Path(tempfile.mkdtemp(prefix='owned-interrupt-',dir=root/'.superpowers'))
                child_code="import subprocess,sys,time,pathlib; child=subprocess.Popen([sys.executable,'-c','import time;time.sleep(30)']);pathlib.Path(sys.argv[1]).write_text(str(child.pid));time.sleep(30)"
                parent_code="import sys;sys.path.insert(0,sys.argv[1]);from lib.fixture_processes import install_interrupt_handlers,wait_owned;install_interrupt_handlers();wait_owned([sys.executable,'-c',sys.argv[3],sys.argv[2]],timeout=20)"
                with (directory/'owner.log').open('wb') as output:
                    owner=subprocess.Popen([sys.executable,'-c',parent_code,str(root/'scripts'),str(directory/'grandchild'),child_code],stdout=output,stderr=output)
                    try:
                        deadline=time.monotonic()+5
                        while not (directory/'grandchild').exists() and time.monotonic()<deadline:time.sleep(.02)
                        self.assertTrue((directory/'grandchild').exists())
                        pid=int((directory/'grandchild').read_text());owner.send_signal(sig)
                        self.assertNotEqual(owner.wait(timeout=6),0)
                        deadline=time.monotonic()+2
                        while time.monotonic()<deadline:
                            state=pathlib.Path('/proc')/str(pid)/'stat'
                            if not state.exists() or state.read_text().split()[2]=='Z':break
                            time.sleep(.02)
                        else:self.fail('grandchild remained live after owned group cleanup')
                    finally:
                        if owner.poll() is None:owner.kill();owner.wait(timeout=2)
    def test_deferred_termination_restores_before_interrupt_propagates(self):
        with self.assertRaises(KeyboardInterrupt):
            install_interrupt_handlers()
            state=[]
            try:
                with defer_interrupts():
                    state.append('stopped')
                    os.kill(os.getpid(),signal.SIGTERM)
            finally:
                with defer_interrupts():state.append('restored')
        self.assertEqual(state,['stopped','restored'])
    def test_signal_delivered_to_worker_is_deferred_through_main_cleanup(self):
        install_interrupt_handlers()
        release=threading.Event();sent=threading.Event();state=[]
        def worker():
            release.wait(timeout=2)
            signal.pthread_kill(threading.get_ident(),signal.SIGTERM)
            sent.set()
        thread=threading.Thread(target=worker);thread.start()
        try:
            with self.assertRaises(KeyboardInterrupt):
                with defer_interrupts():
                    release.set();self.assertTrue(sent.wait(timeout=1));time.sleep(.05)
                    state.append('cleanup-completed')
            self.assertEqual(state,['cleanup-completed'])
        finally:thread.join(timeout=2)
    def test_timeout_closes_owned_group(self):
        directory=pathlib.Path(tempfile.mkdtemp(prefix='owned-timeout-',dir=root/'.superpowers'))
        code="import os,pathlib,sys,time;pathlib.Path(sys.argv[1]).write_text(str(os.getpid()));time.sleep(30)"
        with self.assertRaises(subprocess.TimeoutExpired):
            wait_owned([sys.executable,'-c',code,str(directory/'pid')],timeout=.3)
        pid=int((directory/'pid').read_text())
        self.assertFalse((pathlib.Path('/proc')/str(pid)).exists())
    def test_normal_parent_exit_still_closes_term_ignoring_descendant(self):
        directory=pathlib.Path(tempfile.mkdtemp(prefix='owned-descendant-',dir=root/'.superpowers'))
        descendant="import os,pathlib,signal,sys,time;signal.signal(signal.SIGTERM,signal.SIG_IGN);pathlib.Path(sys.argv[1]).write_text(str(os.getpid()));time.sleep(30)"
        parent="import pathlib,subprocess,sys,time;subprocess.Popen([sys.executable,'-c',sys.argv[2],sys.argv[1]]);deadline=time.monotonic()+2\nwhile not pathlib.Path(sys.argv[1]).exists() and time.monotonic()<deadline:time.sleep(.01)"
        self.assertEqual(wait_owned([sys.executable,'-c',parent,str(directory/'pid'),descendant],timeout=3),0)
        pid=int((directory/'pid').read_text());state=pathlib.Path('/proc')/str(pid)/'stat'
        deadline=time.monotonic()+2
        while state.exists() and state.read_text().split()[2]!='Z' and time.monotonic()<deadline:time.sleep(.02)
        self.assertTrue(not state.exists() or state.read_text().split()[2]=='Z')
if __name__=='__main__':unittest.main()
