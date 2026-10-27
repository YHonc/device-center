import {sameLampPreset} from './lampPresetEdit.js';
export const LAMP_PRESET_PREFIX='lamp-preset-v1:';
/** @typedef {{id:string,name:string,brightness:number,temperature:number}} LampPreset */
/** Convenience values, not health or lighting recommendations. @type {LampPreset[]} */
export const BUILTIN_LAMP_PRESETS=[
  {id:'builtin-reading',name:'阅读',brightness:70,temperature:4000},
  {id:'builtin-focus',name:'专注',brightness:100,temperature:5000},
  {id:'builtin-soft',name:'柔和',brightness:30,temperature:2700}
];
/** @param {unknown} value @returns {LampPreset} */
function validate(value){
  if(!value||typeof value!=='object')throw new Error('预设格式无效');
  const v=/** @type {Record<string,unknown>} */(value);
  if(typeof v.id!=='string'||!v.id||v.id.length>100||typeof v.name!=='string'||!v.name.trim()||v.name.trim().length>32)throw new Error('预设名称需为1–32个字符');
  if(!Number.isInteger(v.brightness)||Number(v.brightness)<1||Number(v.brightness)>100||!Number.isInteger(v.temperature)||Number(v.temperature)<2700||Number(v.temperature)>6500)throw new Error('预设亮度或色温超出设备范围');
  return{id:v.id,name:v.name.trim(),brightness:Number(v.brightness),temperature:Number(v.temperature)};
}
/** @param {string|null} raw @param {string} id @returns {LampPreset} */
export function parseLampPreset(raw,id){
  if(raw===null)throw new Error('该预设已在另一窗口删除');
  let data;try{data=JSON.parse(raw);}catch{throw new Error('本地预设存储损坏，未覆盖原内容');}
  if(data?.version!==1)throw new Error('本地预设版本无效，未覆盖原内容');
  const preset=validate(data.preset);if(preset.id!==id||id.startsWith('builtin-'))throw new Error('本地预设标识不匹配');return preset;
}
/** One enqueue preserves the parent's merge/drain/readback and close route.
 * @param {LampPreset} preset @param {(change:{brightness:number,temperature:number},immediate:boolean)=>void} enqueue */
export function enqueueLampPreset(preset,enqueue){const p=validate(preset);enqueue({brightness:p.brightness,temperature:p.temperature},true);}
/** @param {{sampledAt?:number,stale?:boolean}|null} reading @param {number} now */
export const freshLampReading=(reading,now)=>!!reading?.sampledAt&&!reading.stale&&now-reading.sampledAt<=300000;
/** @param {LampPreset} preset @param {{brightness:number,temperature:number,sampledAt:number,stale?:boolean}|null} confirmed @param {{now:number,busy?:boolean,pending?:boolean,disabled?:boolean}} gate */
export const lampPresetMatches=(preset,confirmed,gate)=>!gate.busy&&!gate.pending&&!gate.disabled&&freshLampReading(confirmed,gate.now)&&confirmed?.brightness===preset.brightness&&confirmed?.temperature===preset.temperature;
/** @param {LampPreset[]} presets */
export function defaultLampPresetName(presets){const names=new Set(presets.map(p=>p.name));let n=1;while(names.has(`自定义 ${n}`))n++;return`自定义 ${n}`;}
/** Independent keys avoid replacing another window's new preset. Editors can
 * check the opened item's value before changing it; localStorage has no atomic CAS.
 * @param {{storage:Pick<Storage,'getItem'|'setItem'|'removeItem'|'key'|'length'>,notify?:()=>void,createId?:()=>string}} options */
export function createLampPresetStore({storage,notify=()=>{},createId=()=>crypto.randomUUID()}){
  /** @type {{presets:LampPreset[],error:string}} */let state={presets:[],error:''};
  /** @type {Set<(state:{presets:LampPreset[],error:string})=>void>} */const listeners=new Set();
  const publish=()=>listeners.forEach(fn=>fn({presets:state.presets.map(p=>({...p})),error:state.error}));
  function refresh(){
    /** @type {LampPreset[]} */const presets=[];let error='';
    try{const keys=Array.from({length:storage.length},(_,i)=>storage.key(i)).filter(k=>k?.startsWith(LAMP_PRESET_PREFIX));
      for(const key of keys){if(!key)continue;try{const raw=storage.getItem(key);if(raw!==null)presets.push(parseLampPreset(raw,key.slice(LAMP_PRESET_PREFIX.length)));}catch(e){error=e instanceof Error?e.message:'读取本地预设失败';}}
      state={presets:presets.sort((a,b)=>a.name.localeCompare(b.name,'zh-CN')),error};
    }catch(e){state={...state,error:e instanceof Error?e.message:'读取本地预设失败'};}publish();
  }
  /** @param {()=>void} write */
  function commit(write){try{write();refresh();notify();}catch(e){state={...state,error:e instanceof Error?e.message:'保存本地预设失败'};publish();throw e;}}
  /** @param {LampPreset} p */const write=p=>storage.setItem(LAMP_PRESET_PREFIX+p.id,JSON.stringify({version:1,preset:validate(p)}));
  refresh();
  return{
    refresh,
    /** @param {(state:{presets:LampPreset[],error:string})=>void} fn */
    subscribe(fn){listeners.add(fn);fn({presets:state.presets.map(p=>({...p})),error:state.error});return()=>{listeners.delete(fn);};},
    /** @param {string} name @param {{brightness:number,temperature:number}} reading */
    save(name,reading){const p=validate({id:createId(),name,...reading});commit(()=>write(p));return p.id;},
    /** Save stable confirmed values only, never a slider preview during debounce/drain.
     * @param {{brightness:number,temperature:number,sampledAt:number,stale?:boolean}|null} confirmed
     * @param {{now:number,busy?:boolean,pending?:boolean,disabled?:boolean}} gate */
    saveCurrent(confirmed,gate){if(gate.busy||gate.pending||gate.disabled||!confirmed||!freshLampReading(confirmed,gate.now))return null;refresh();const p=validate({id:createId(),name:defaultLampPresetName(state.presets),brightness:confirmed.brightness,temperature:confirmed.temperature});commit(()=>write(p));return p;},
    /** @param {string} id @param {string} name @param {LampPreset} [expected] */
    rename(id,name,expected){commit(()=>{const p=parseLampPreset(storage.getItem(LAMP_PRESET_PREFIX+id),id);if(expected&&!sameLampPreset(p,expected))throw new Error('该预设已在另一窗口更新，请重新编辑');write(validate({...p,name}));});},
    /** @param {string} id @param {LampPreset} [expected] */
    remove(id,expected){commit(()=>{if(expected){const p=parseLampPreset(storage.getItem(LAMP_PRESET_PREFIX+id),id);if(!sameLampPreset(p,expected))throw new Error('该预设已在另一窗口更新，请重新编辑');}storage.removeItem(LAMP_PRESET_PREFIX+id);});}
  };
}
