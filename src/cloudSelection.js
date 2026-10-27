/** @typedef {{id:string,model:string,ip:string|null,eligible:boolean}} Candidate */
/**
 * Choose only a uniquely identified supported lamp; array order has no meaning.
 * @param {Candidate[]} devices
 * @param {string|null|undefined} savedIp
 * @param {string|null|undefined} expectedIp
 * @returns {{deviceId:string|null,reason:'saved'|'expected'|'unique'|'ambiguous'|'missing-target'|'none'}}
 */
export function selectAutomaticLamp(devices,savedIp,expectedIp) {
  const lamps=devices.filter(d=>d.eligible&&d.model==='yeelink.light.lamp22');
  const saved=savedIp?.trim();const expected=expectedIp?.trim();
  if(saved){const matches=lamps.filter(d=>d.ip===saved);if(matches.length===1)return{deviceId:matches[0].id,reason:'saved'};if(matches.length>1)return{deviceId:null,reason:'ambiguous'};}
  if(expected){const matches=lamps.filter(d=>d.ip===expected);return matches.length===1?{deviceId:matches[0].id,reason:'expected'}:{deviceId:null,reason:matches.length?'ambiguous':'missing-target'};}
  if(saved)return{deviceId:null,reason:'missing-target'};
  return lamps.length===1?{deviceId:lamps[0].id,reason:'unique'}:{deviceId:null,reason:lamps.length?'ambiguous':'none'};
}
/** @param {number} expected @param {number} current @param {boolean} disposed */
export const isCurrentAttempt=(expected,current,disposed)=>!disposed&&expected===current;
