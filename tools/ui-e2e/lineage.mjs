import assert from 'node:assert/strict';

const identity = (p) => `${p.pid}:${p.created}`;
export class OwnedTree {
  constructor(pid, baseline = []) {
    assert.ok(Number.isSafeInteger(pid) && pid > 0);
    this.pid = pid;
    this.seen = new Map();
    this.external = new Map(baseline.map((p) => [identity(p), { ...p, last_seen: Date.now() }]));
    this.ambiguous = new Map();
  }
  observe(processes) {
    const now = Date.now();
    const live = new Map(processes.map((p) => [identity(p), p]));
    if (!this.root) {
      const root = processes.find((p) => p.pid === this.pid);
      if (!root) throw new Error('Owned application identity was not observed');
      this.root = identity(root);
      this.seen.set(this.root, { ...root, last_seen: now });
    }
    let changed = true;
    while (changed) {
      changed = false;
      for (const p of processes) {
        if (this.external.has(identity(p))) continue;
        assert.ok(Number.isFinite(Date.parse(p.created)), 'Missing process creation identity');
        if (this.seen.has(identity(p))) continue;
        const parent = [...this.seen.values()].find(
          (owner) =>
            owner.pid === p.parent &&
            Date.parse(p.created) >= Date.parse(owner.created) &&
            (processes.some((current) => identity(current) === identity(owner)) ||
              Date.parse(p.created) <= owner.last_seen),
        );
        if (parent) {
          this.seen.set(identity(p), { ...p, last_seen: now });
          changed = true;
        }
      }
    }
    // Only exclude foreign children whose parent identity is still present.
    // A new orphan across a sampling gap cannot safely be attributed or killed.
    changed = true;
    while (changed) {
      changed = false;
      for (const p of processes) {
        if (this.seen.has(identity(p)) || this.external.has(identity(p))) continue;
        const parent = [...this.external.values()].find(
          (e) =>
            e.pid === p.parent &&
            live.has(identity(e)) &&
            Date.parse(p.created) >= Date.parse(e.created),
        );
        if (parent) {
          this.external.set(identity(p), { ...p, last_seen: now });
          changed = true;
        }
      }
    }
    for (const p of processes) {
      if (this.seen.has(identity(p)) || this.external.has(identity(p))) continue;
      // Conservatively retain all new orphan/opaque chains; never terminate them.
      this.ambiguous.set(identity(p), p);
    }
    const active = processes.filter((p) => this.seen.has(identity(p)));
    for (const p of active) this.seen.set(identity(p), { ...p, last_seen: now });
    return active;
  }
  assertKnown() {
    if (this.ambiguous.size)
      throw new Error('Process ownership crossed an observation gap; cleanup cannot be confirmed');
  }
}
