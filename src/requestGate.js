// Each request owns its result only until another request or device scope starts.
export function createRequestGate() {
  let generation = 0;
  return {
    begin() { const current = ++generation; return () => current === generation; },
    invalidate() { generation++; },
  };
}
