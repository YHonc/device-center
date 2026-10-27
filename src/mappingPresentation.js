/** A physical slot remains stable even when its displayed key mapping changes. */
/** @param {string} physicalId @param {string|null|undefined} currentAction @param {(id:string)=>string} label */
export function keycapLabel(physicalId,currentAction,label) {
  return label(currentAction?.startsWith('key:')?currentAction.slice(4):physicalId);
}
