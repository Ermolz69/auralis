import assert from 'node:assert/strict';

const identity = (p) => `${p.pid}:${p.created}`;
export class OwnedTree {
  constructor(pid) {
    assert.ok(Number.isSafeInteger(pid) && pid > 0);
    this.pid = pid;
    this.seen = new Map();
  }
  observe(processes) {
    const now = Date.now();
    if (!this.root) {
      const root = processes.find(p => p.pid === this.pid);
      if (!root) throw new Error('Owned application identity was not observed');
      this.root = identity(root);
      this.seen.set(this.root, { ...root, last_seen: now });
    }
    let changed = true;
    while (changed) {
      changed = false;
      for (const p of processes) {
        assert.ok(Number.isFinite(Date.parse(p.created)), 'Missing process creation identity');
        if (this.seen.has(identity(p))) continue;
        const parent = [...this.seen.values()].find(owner => owner.pid === p.parent &&
          Date.parse(p.created) >= Date.parse(owner.created) &&
          (processes.some(current => identity(current) === identity(owner)) || Date.parse(p.created) <= owner.last_seen));
        if (parent) {
          this.seen.set(identity(p), { ...p, last_seen: now });
          changed = true;
        }
      }
    }
    const active = processes.filter(p => this.seen.has(identity(p)));
    for (const p of active) this.seen.set(identity(p), { ...p, last_seen: now });
    return active;
  }
}
