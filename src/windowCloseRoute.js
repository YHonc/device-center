/** Compact must never authorize application exit or bypass the main window's dirty-state confirmation.
 * @param {{exit?:boolean}|null|undefined} request
 * @returns {'close_device_window'|null}
 */
export function compactCloseAction(request){return request?.exit===false?'close_device_window':null;}
/** @param {boolean} pending @param {()=>Promise<void>} flush @param {()=>boolean} failed @param {()=>Promise<unknown>} close */
export async function finishCompactClose(pending,flush,failed,close){if(pending){await flush();if(failed())return false;}await close();return true;}
