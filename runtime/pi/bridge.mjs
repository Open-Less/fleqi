import { randomUUID } from 'node:crypto';

export class Bridge {
  constructor(send) { this.send = send; this.pending = new Map(); }
  request(type, data = {}, signal) {
    const id = randomUUID();
    return new Promise((resolve, reject) => {
      const abort = () => { this.pending.delete(id); reject(new Error('操作已取消')); };
      if (signal?.aborted) return abort();
      this.pending.set(id, {
        resolve: (value) => { signal?.removeEventListener('abort', abort); resolve(value); },
        reject: (error) => { signal?.removeEventListener('abort', abort); reject(error); },
      });
      signal?.addEventListener('abort', abort, { once: true });
      this.send({ id, type, ...data });
    });
  }
  receive(message) {
    const request = this.pending.get(message.id);
    if (!request) return false;
    this.pending.delete(message.id);
    if (message.error) request.reject(new Error(message.error));
    else request.resolve(message.result);
    return true;
  }
}

// Each process handles one connection. Rust serializes processes that share a
// profile, so OAuth refresh cannot overwrite another process's rotated token.
export class NativeCredentials {
  constructor(bridge,providerId) { this.bridge = bridge; this.providerId=providerId; this.queue = Promise.resolve(); }
  read(provider, options) { if(provider!==this.providerId)return Promise.resolve(undefined);return this.bridge.request('credential_read', {}, options?.signal).then(v => v ?? undefined); }
  async list() { const value = await this.read(this.providerId); return value ? [{ providerId:this.providerId, type: value.type }] : []; }
  modify(provider, fn, options) {
    if(provider!==this.providerId)return Promise.reject(new Error('不允许修改其他连接的凭据'));
    const next = this.queue.then(async () => {
      const current = await this.read(provider, options);
      const value = await fn(current);
      if (value !== undefined) await this.bridge.request('credential_write', { credential: value }, options?.signal);
      return value ?? current;
    });
    this.queue = next.catch(() => {});
    return next;
  }
  delete(provider, options) {
    if(provider!==this.providerId)return Promise.reject(new Error('不允许删除其他连接的凭据'));
    const next = this.queue.then(() => this.bridge.request('credential_delete', {}, options?.signal));
    this.queue = next.catch(() => {});
    return next;
  }
}
