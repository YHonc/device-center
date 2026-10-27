/** @param {number} value @param {{min:number,max:number,stepSegments:{until:number,step:number}[]}} capability */
export function snapDpi(value,capability){
  const bounded=Math.max(capability.min,Math.min(capability.max,value));
  const step=capability.stepSegments.find(segment=>bounded<=segment.until)?.step;
  if(!step)return bounded;
  return Math.max(capability.min,Math.min(capability.max,Math.round(bounded/step)*step));
}

/** Navigate the next legal sensor value, including boundaries between step segments.
 * @param {number} value @param {number} direction @param {{min:number,max:number,stepSegments:{until:number,step:number}[]}} capability
 */
export function nextDpi(value,direction,capability){
  const candidates=[];
  let previous=capability.min-1;
  for(const segment of capability.stepSegments){
    const lower=Math.max(capability.min,previous+1),upper=Math.min(capability.max,segment.until);
    const candidate=direction>0?Math.max(Math.ceil(lower/segment.step)*segment.step,(Math.floor(value/segment.step)+1)*segment.step):Math.min(Math.floor(upper/segment.step)*segment.step,(Math.ceil(value/segment.step)-1)*segment.step);
    if(candidate>=lower&&candidate<=upper)candidates.push(candidate);
    previous=segment.until;
  }
  return candidates.length?(direction>0?Math.min(...candidates):Math.max(...candidates)):value;
}

/** @param {string} key @param {number} value @param {{min:number,max:number,stepSegments:{until:number,step:number}[]}} capability */
export function dpiKeyboardValue(key,value,capability){
  if(key==='Home')return nextDpi(capability.min-1,1,capability);
  if(key==='End')return nextDpi(capability.max+1,-1,capability);
  if(['ArrowRight','ArrowUp'].includes(key))return nextDpi(value,1,capability);
  if(['ArrowLeft','ArrowDown'].includes(key))return nextDpi(value,-1,capability);
  if(key==='PageUp'||key==='PageDown'){
    for(let i=0;i<10;i++)value=nextDpi(value,key==='PageUp'?1:-1,capability);
    return value;
  }
  return null;
}

/** Native number editing keeps caret/navigation shortcuts; the range has no text caret.
 * @param {{key:string,altKey?:boolean,ctrlKey?:boolean,metaKey?:boolean,shiftKey?:boolean}} event
 * @param {'range'|'number'} inputType
 */
export function isDpiAdjustmentKey(event,inputType){
  if(event.altKey||event.ctrlKey||event.metaKey||event.shiftKey)return false;
  return (inputType==='number'?['ArrowUp','ArrowDown','PageUp','PageDown']:['ArrowUp','ArrowDown','ArrowLeft','ArrowRight','Home','End','PageUp','PageDown']).includes(event.key);
}
