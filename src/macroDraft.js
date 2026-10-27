const keyCodes = {Enter:40,Escape:41,Backspace:42,Tab:43,Space:44,Minus:45,Equal:46,BracketLeft:47,BracketRight:48,Backslash:49,Semicolon:51,Quote:52,Backquote:53,Comma:54,Period:55,Slash:56,CapsLock:57,PrintScreen:70,ScrollLock:71,Pause:72,Insert:73,Home:74,PageUp:75,Delete:76,End:77,PageDown:78,ArrowRight:79,ArrowLeft:80,ArrowDown:81,ArrowUp:82};
const modifiers={ControlLeft:224,ShiftLeft:225,AltLeft:226,MetaLeft:227,ControlRight:228,ShiftRight:229,AltRight:230,MetaRight:231};
/** @typedef {{kind:string,state:string,code:number,delayMs:number}} MacroAction */
/** @param {MacroAction[]} actions @param {{kind:string,state:string,code:number}} value @param {number} now @param {number} lastTime */
export function appendRecordedAction(actions,value,now,lastTime) {
  if(actions.length) actions[actions.length-1].delayMs=Math.max(1,Math.min(65535,Math.round(now-lastTime)));
  actions.push({...value,delayMs:0});
  return now;
}
/** Reserve Escape and Tab for leaving capture; these keys remain available in the action editor.
 * @param {{code:string,preventDefault:()=>void,stopPropagation:()=>void}} event @param {()=>void} stop
 */
export function stopRecordingWithKey(event,stop){
  if(event.code!=='Escape'&&event.code!=='Tab')return false;
  if(event.code==='Escape'){event.preventDefault();event.stopPropagation();}
  stop();
  return true;
}
/** @param {{text:()=>Promise<string>}} file @param {()=>boolean} current @param {number} maxActions */
export async function importMacroFile(file,current,maxActions=70){
  const text=await file.text();
  return current()?parseMacro(text,maxActions):null;
}
const specialNames=['Num Lock','小键盘 /','小键盘 *','小键盘 -','小键盘 +','小键盘 Enter','小键盘 1','小键盘 2','小键盘 3','小键盘 4','小键盘 5','小键盘 6','小键盘 7','小键盘 8','小键盘 9','小键盘 0','小键盘 .','国际键 \\','应用菜单','电源','小键盘 =','F13','F14','F15','F16','F17','F18','F19','F20','F21','F22','F23','F24'];
/** @param {string} kind */
export function macroKeyOptions(kind){
  if(kind==='modifier')return ['左 Ctrl','左 Shift','左 Alt','左 Win','右 Ctrl','右 Shift','右 Alt','右 Win'].map((label,i)=>({code:224+i,label}));
  if(kind==='mouse')return [1,2,4,8,16].map((code,i)=>({code,label:['左键','右键','中键','侧键 1','侧键 2'][i]}));
  return Array.from({length:112},(_,i)=>{const code=i+4;const named=Object.entries(keyCodes).find(([,value])=>value===code)?.[0];return {code,label:code<30?String.fromCharCode(code+61):code<40?(code===39?'0':String(code-29)):code>=58&&code<=69?`F${code-57}`:code>=83?specialNames[code-83]:named??`HID ${code}`};});
}
/** @param {string} code */
export function macroKey(code) {
  const modifier=/** @type {Record<string,number>} */(modifiers)[code];
  if (modifier) return {kind:'modifier',code:modifier};
  if (/^Key[A-Z]$/.test(code)) return {kind:'key',code:code.charCodeAt(3)-61};
  if (/^Digit[0-9]$/.test(code)) return {kind:'key',code:code==='Digit0'?39:Number(code[5])+29};
  if (/^F([1-9]|1[0-2])$/.test(code)) return {kind:'key',code:Number(code.slice(1))+57};
  const key=/** @type {Record<string,number>} */(keyCodes)[code];
  return key?{kind:'key',code:key}:null;
}
/** @param {unknown} actions */
export function validateMacro(actions,maxActions=70) {
  if (!Array.isArray(actions)||actions.length>maxActions) throw new Error(`动作最多 ${maxActions} 条。`);
  const pressed=new Set();
  for (const [index,action] of actions.entries()) {
    if (!action||!['key','modifier','mouse'].includes(action.kind)||!['down','up'].includes(action.state)||!Number.isInteger(action.code)||!Number.isInteger(action.delayMs)||action.delayMs<0||action.delayMs>65535) throw new Error('动作类型、键码或延时无效。');
    if(index<actions.length-1&&action.delayMs<1)throw new Error('非末动作的延迟至少为 1 毫秒。');
    if ((action.kind==='key'&&(action.code<4||action.code>115))||(action.kind==='modifier'&&(action.code<224||action.code>231))||(action.kind==='mouse'&&![1,2,4,8,16].includes(action.code))) throw new Error('键码超出支持范围。');
    const key=`${action.kind}:${action.code}`;
    if(action.state==='down'){if(pressed.has(key))throw new Error('同一按键重复按下，请先补齐抬起动作。');pressed.add(key);}
    else{if(!pressed.delete(key))throw new Error('抬起动作缺少对应按下动作。');}
  }
  if(pressed.size)throw new Error('宏末尾仍有按键未抬起。');
  return actions.map(({kind,state,code,delayMs})=>({kind,state,code,delayMs}));
}
/** @param {string} text */
export function parseMacro(text,maxActions=70){const value=JSON.parse(text);return validateMacro(Array.isArray(value)?value:value?.actions,maxActions);}
