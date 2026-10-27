const keyboardPerformance=new Set(['sleepSeconds','reportRateHz','lowLatency','macMode','winLock']);
const mousePerformance=new Set(['reportRateHz','dpiLevel']);
/** @template {{key:string}} T @param {T[]} fields @param {boolean} keyboard */
export function performanceFields(fields,keyboard){
  const keys=keyboard?keyboardPerformance:mousePerformance;
  return fields.filter(field=>keys.has(field.key));
}
/** @param {{key?:string,options?:{value:unknown,label:string}[]}|undefined} field @param {unknown} value */
export function settingValueLabel(field,value){
  if(value===null||value===undefined)return '未知';
  const option=field?.options?.find(option=>option.value===value);
  if(option)return option.label;
  if(value===true)return '开启';
  if(value===false)return '关闭';
  if(field?.key==='sleepSeconds'&&value===0)return '永不休眠';
  return String(value);
}
