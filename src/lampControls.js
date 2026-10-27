/** Sync controls from the latest accepted readback after a drain. Never replace
 * a newer queued preview, or bypass snapshot ordering with a rejected response.
 * @param {{brightness?:number,temperature?:number}|null} confirmed
 * @param {{brightness?:number,temperature?:number}} pending
 * @returns {{brightness?:number,temperature?:number}} */
export function settledLampControls(confirmed,pending){
  if(!confirmed)return{};
  return{
    ...(pending.brightness===undefined&&confirmed.brightness!==undefined?{brightness:confirmed.brightness}:{}),
    ...(pending.temperature===undefined&&confirmed.temperature!==undefined?{temperature:confirmed.temperature}:{})
  };
}
