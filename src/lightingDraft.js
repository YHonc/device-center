/** A mode change preserves its target slot; target parameters are read only after applying.
 * @param {{mode:number,brightness:number|null,speed:number|null,multiColor:boolean|null,rgb:number[]|null}|null} snapshot
 * @param {{mode:number,brightness:number|null,speed:number|null,multiColor:boolean|null,rgb:number[]|null}} draft
 * @param {{brightnessEditable:boolean,speedEditable:boolean,colorEditable:boolean}|undefined} selected
 */
export function lightingChanges(snapshot,draft,selected){
  /** @type {Record<string,number|boolean|number[]>} */
  const changes={};
  if(!snapshot)return changes;
  if(draft.mode!==snapshot.mode)return {mode:draft.mode};
  if(selected?.brightnessEditable&&draft.brightness!==null&&draft.brightness!==snapshot.brightness)changes.brightness=draft.brightness;
  if(selected?.speedEditable&&draft.speed!==null&&draft.speed!==snapshot.speed)changes.speed=draft.speed;
  if(selected?.colorEditable&&draft.multiColor!==null&&draft.multiColor!==snapshot.multiColor)changes.multiColor=draft.multiColor;
  if(selected?.colorEditable&&draft.multiColor===false&&draft.rgb&&snapshot.rgb&&JSON.stringify(draft.rgb)!==JSON.stringify(snapshot.rgb))changes.rgb=draft.rgb;
  return changes;
}
