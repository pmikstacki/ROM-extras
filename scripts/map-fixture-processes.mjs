import { spawn } from 'node:child_process';

function bounded(promise, milliseconds, message) {
  let timer;
  return Promise.race([
    promise,
    new Promise((_, reject) => { timer = setTimeout(() => reject(Error(message)), milliseconds); }),
  ]).finally(() => clearTimeout(timer));
}

/** Own separate Unix groups; never find or terminate unrelated processes by port or name. */
export class FixtureProcesses {
  #records = new Map();
  #closing;
  start(command, args, options = {}) {
    if (process.platform === 'win32') throw Error('Controlled fixture runner requires Unix process groups');
    if (this.#closing) throw Error('Controlled fixture scope is closed');
    const child = spawn(command, args, { ...options, detached: true, stdio: options.stdio ?? ['ignore', 'pipe', 'inherit'] });
    const finished = new Promise(resolve => {
      child.once('error', () => resolve({ failed: true, code: null }));
      child.once('close', code => resolve({ failed: false, code }));
    });
    this.#records.set(child, finished);
    return child;
  }
  async readyLine(child, milliseconds = 5000) {
    if (!this.#records.has(child) || !child.stdout) throw Error('Controlled fixture readiness output unavailable');
    let bytes = Buffer.alloc(0);
    let data;
    const line = new Promise((resolve, reject) => {
      data = chunk => {
        if (bytes.length + chunk.length > 1024) { reject(Error('Controlled fixture readiness exceeded limit')); return; }
        bytes = Buffer.concat([bytes, chunk]);
        const end = bytes.indexOf(10);
        if (end !== -1) resolve(bytes.subarray(0, end).toString('utf8').trim());
      };
      child.stdout.on('data', data);
    });
    const stopped = this.#records.get(child).then(() => { throw Error('Controlled fixture closed before readiness'); });
    try {
      return await bounded(Promise.race([line, stopped]), milliseconds, 'Controlled fixture readiness deadline');
    } finally {
      child.stdout.off('data', data);
      child.stdout.resume();
    }
  }
  async wait(child, milliseconds = 90000) {
    if (!this.#records.has(child)) throw Error('Controlled fixture process is not owned');
    const outcome = await bounded(this.#records.get(child), milliseconds, 'Controlled fixture process deadline');
    if (outcome.failed || outcome.code !== 0) throw Error('Controlled fixture process failed');
  }
  close() {
    this.#closing ??= this.#shutdown();
    return this.#closing;
  }
  #signal(child, signal) {
    if (!child.pid) return;
    try { process.kill(-child.pid, signal); }
    catch (error) { if (error.code !== 'ESRCH') throw Error('Controlled fixture group cleanup failed'); }
  }
  async #shutdown() {
    const children = [...this.#records.keys()].reverse();
    for (const child of children) this.#signal(child, 'SIGTERM');
    try {
      await Promise.all(children.map(child => bounded(this.#records.get(child), 5000, 'Controlled fixture shutdown deadline').catch(() => null)));
    } finally {
      // A parent can exit before its preview children. Finish the owned group too.
      for (const child of children) this.#signal(child, 'SIGKILL');
    }
    await Promise.all(children.map(child => bounded(this.#records.get(child), 1000, 'Controlled fixture did not close')));
  }
}
