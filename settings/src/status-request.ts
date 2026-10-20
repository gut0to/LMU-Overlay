/** Concurrent callers observe the same host snapshot instead of stale local state. */
export class StatusRequest {
  private active: Promise<boolean> | null = null;

  run(fetch: () => Promise<boolean>): Promise<boolean> {
    if (this.active) return this.active;
    const request = Promise.resolve().then(fetch).finally(() => { this.active = null; });
    this.active = request;
    return request;
  }
}
