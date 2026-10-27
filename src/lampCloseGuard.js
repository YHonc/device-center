/** @param {{queued:number,busy:boolean,draining:boolean}} state */
export const lampNeedsCloseConfirmation=state=>state.queued>0||state.busy||state.draining;
/** @param {{busy:boolean,draining:boolean}} state */
export const lampCanDiscardQueue=state=>!state.busy&&!state.draining;
