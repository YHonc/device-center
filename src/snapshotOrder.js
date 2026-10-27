/** @typedef {{updatedAt?:number,sampledAt?:number}} TimedReading */
/** Order device caches independently. A snapshot requested before an event cannot replace that event. */
export function createSnapshotOrder(){
  /** @type {Record<string,{time:number,event:number}>} */ const latest={};
  return {
    capture(){return Object.fromEntries(Object.entries(latest).map(([key,v])=>[key,v.event]));},
    /** @param {string} key @param {TimedReading} value @param {'event'|'snapshot'|'direct'} source @param {Record<string,number>} [captured] @param {number} [fallback] */
    accept(key,value,source,captured={},fallback=Date.now()){
      const old=latest[key];
      if(source==='snapshot'&&old&&(captured[key]??0)<old.event)return false;
      const time=value.updatedAt??value.sampledAt??fallback;
      if(old&&time<old.time)return false;
      latest[key]={time,event:(old?.event??0)+(source==='event'?1:0)};return true;
    }
  };
}
