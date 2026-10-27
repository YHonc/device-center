import test from 'node:test';
import assert from 'node:assert/strict';
import {LAMP_PRESET_PREFIX,BUILTIN_LAMP_PRESETS,parseLampPreset,createLampPresetStore,enqueueLampPreset,freshLampReading,lampPresetMatches} from '../src/lampPresets.js';
import {settledLampControls} from '../src/lampControls.js';
import {createSnapshotOrder} from '../src/snapshotOrder.js';
import {finishCompactClose} from '../src/windowCloseRoute.js';
const memory=()=>{const data=new Map();return{get length(){return data.size;},key:i=>[...data.keys()][i]??null,getItem:k=>data.get(k)??null,setItem:(k,v)=>data.set(k,v),removeItem:k=>data.delete(k)};};
test('损坏存储拒绝覆盖，合法整数4809K保存且白名单不含凭据',()=>{
 const storage=memory();storage.setItem(LAMP_PRESET_PREFIX+'bad','broken');const store=createLampPresetStore({storage,createId:()=> 'a'});assert.throws(()=>store.rename('bad','灯'));assert.equal(storage.getItem(LAMP_PRESET_PREFIX+'bad'),'broken');
 assert.throws(()=>parseLampPreset(JSON.stringify({version:1,preset:{id:'a',name:'x',brightness:101,temperature:4809}}),'a'));
 store.save('  我的阅读  ',{brightness:73,temperature:4809,token:'must-not-save',sampledAt:1,power:true});const saved=JSON.parse(storage.getItem(LAMP_PRESET_PREFIX+'a')).preset;assert.deepEqual(saved,{id:'a',name:'我的阅读',brightness:73,temperature:4809});assert.throws(()=>store.rename('a',' '));assert.throws(()=>store.rename('a','x'.repeat(33)));
});
test('双窗口目录增改删同步只刷新目录，无回环且操作重读最新集合',()=>{
 const storage=memory();let a,b;let aState,bState;let notices=0;
 a=createLampPresetStore({storage,createId:()=> 'a',notify:()=>{notices++;b.refresh();}});b=createLampPresetStore({storage,createId:()=> 'b',notify:()=>{notices++;a.refresh();}});
 const offA=a.subscribe(s=>aState=s),offB=b.subscribe(s=>bState=s);
 a.save('A',{brightness:10,temperature:2700});assert.equal(bState.presets[0].name,'A');b.save('B',{brightness:20,temperature:6500,power:false});assert.equal(aState.presets.length,2);a.rename('b','新名');assert.equal(bState.presets.find(p=>p.id==='b').name,'新名');b.remove('a');assert.deepEqual(aState.presets.map(p=>p.id),['b']);assert.equal(notices,4);offA();offB();
});
test('存储写失败不会宣称保存成功或通知另一窗口',()=>{let notified=false;const storage=memory();storage.setItem=()=>{throw new Error('quota');};const store=createLampPresetStore({storage,notify:()=>notified=true});let state;store.subscribe(s=>state=s);assert.throws(()=>store.save('测试',{brightness:20,temperature:4000}));assert.equal(state.presets.length,0);assert.equal(state.error,'quota');assert.equal(notified,false);});
test('两个窗口交错保存不同预设，不会用旧目录覆盖另一项',()=>{const storage=memory();const rawSet=storage.setItem;let intercept=true;let b;storage.setItem=(key,value)=>{if(intercept){intercept=false;b.save('B',{brightness:20,temperature:4809});}rawSet(key,value);};const a=createLampPresetStore({storage,createId:()=> 'a'});b=createLampPresetStore({storage,createId:()=> 'b'});a.save('A',{brightness:50,temperature:4000});a.refresh();let state;a.subscribe(s=>state=s);assert.deepEqual(state.presets.map(p=>p.id).sort(),['a','b']);});
test('一击保存稳定确认值并自动命名，待发/通信/关闭/过期时不保存',()=>{
 const storage=memory();let serial=0;const store=createLampPresetStore({storage,createId:()=>`auto-${++serial}`});const confirmed={brightness:67,temperature:4809,sampledAt:100};
 for(const gate of [{now:101,pending:true},{now:101,busy:true},{now:101,disabled:true},{now:400001}])assert.equal(store.saveCurrent(confirmed,gate),null);
 assert.equal(store.saveCurrent(null,{now:101}),null);assert.equal(storage.length,0);
 const first=store.saveCurrent(confirmed,{now:101});assert.equal(first.name,'自定义 1');assert.deepEqual(parseLampPreset(storage.getItem(LAMP_PRESET_PREFIX+first.id),first.id),{id:first.id,name:'自定义 1',brightness:67,temperature:4809});
 const second=store.saveCurrent(confirmed,{now:102});assert.equal(second.name,'自定义 2');store.rename(first.id,'阅读设置');const third=store.saveCurrent(confirmed,{now:103});assert.equal(third.name,'自定义 1');assert.equal(storage.length,3);
});
test('点击预设只排队，匹配态须等确认读数且不能沿用pending或过期状态',()=>{const preset=BUILTIN_LAMP_PRESETS[0];let queued;let confirmed={brightness:30,temperature:2700,sampledAt:100};enqueueLampPreset(preset,c=>queued=c);assert.deepEqual(queued,{brightness:70,temperature:4000});assert.equal(lampPresetMatches(preset,confirmed,{now:101}),false);confirmed={...queued,sampledAt:102};assert.equal(lampPresetMatches(preset,confirmed,{now:103,pending:true}),false);assert.equal(lampPresetMatches(preset,confirmed,{now:103,busy:true}),false);assert.equal(lampPresetMatches(preset,confirmed,{now:103}),true);assert.equal(lampPresetMatches(preset,confirmed,{now:400000}),false);});
test('预设一次合并入既有队列，默认不隐式切电源，关闭沿用flush结果',async()=>{
 let pending={brightness:90},calls=0;enqueueLampPreset(BUILTIN_LAMP_PRESETS[0],(change,immediate)=>{pending={...pending,...change};calls++;assert.equal(immediate,true);});assert.equal(calls,1);assert.deepEqual(pending,{brightness:70,temperature:4000});
 let closed=false;await finishCompactClose(true,async()=>{pending={};},()=>false,async()=>{closed=true;});assert.equal(closed,true);
 closed=false;await finishCompactClose(true,async()=>{},()=>true,async()=>{closed=true;});assert.equal(closed,false);
 const sent=[];enqueueLampPreset({id:'off',name:'灯光',brightness:20,temperature:4809,power:true},c=>sent.push(c));assert.deepEqual(sent,[{brightness:20,temperature:4809}]);
});
test('陈旧或未知状态不能作为可保存读数；事件先于direct时收尾以accepted值为准',()=>{
 assert.equal(freshLampReading(null,500000),false);assert.equal(freshLampReading({sampledAt:1},500000),false);assert.equal(freshLampReading({sampledAt:499999,stale:true},500000),false);assert.equal(freshLampReading({sampledAt:499999},500000),true);
 const order=createSnapshotOrder();let confirmed={brightness:50,temperature:4000,sampledAt:100};const event={brightness:70,temperature:4809,sampledAt:200,updatedAt:210};assert.equal(order.accept('lamp',event,'event'),true);confirmed=event;
 const direct={brightness:70,temperature:4800,sampledAt:200};assert.equal(order.accept('lamp',direct,'direct'),false);assert.deepEqual(settledLampControls(confirmed,{}),{brightness:70,temperature:4809});assert.deepEqual(settledLampControls(confirmed,{brightness:90}),{temperature:4809});
});
test('离线名称编辑不应用灯光；跨窗改名后旧编辑不能覆盖或删除新值',()=>{
 const storage=memory();const a=createLampPresetStore({storage,createId:()=> 'a'});const b=createLampPresetStore({storage});
 a.save('原名称',{brightness:67,temperature:4809});const expected=parseLampPreset(storage.getItem(LAMP_PRESET_PREFIX+'a'),'a');
 b.rename('a','另一窗口名称');assert.throws(()=>a.rename('a','迟到草稿',expected),/另一窗口更新/);assert.throws(()=>a.remove('a',expected),/另一窗口更新/);
 assert.equal(parseLampPreset(storage.getItem(LAMP_PRESET_PREFIX+'a'),'a').name,'另一窗口名称');
 const latest=parseLampPreset(storage.getItem(LAMP_PRESET_PREFIX+'a'),'a');a.rename('a','离线改名',latest);assert.equal(parseLampPreset(storage.getItem(LAMP_PRESET_PREFIX+'a'),'a').name,'离线改名');
});
test('另一窗口删除后旧名称编辑不能复活预设或改动其他项',()=>{
 const storage=memory();let serial=0;const store=createLampPresetStore({storage,createId:()=>`p${++serial}`});store.save('A',{brightness:20,temperature:3000});store.save('B',{brightness:50,temperature:4000});
 const expected=parseLampPreset(storage.getItem(LAMP_PRESET_PREFIX+'p1'),'p1');store.remove('p1');assert.throws(()=>store.rename('p1','迟到重命名',expected),/另一窗口删除/);assert.equal(storage.length,1);assert.equal(parseLampPreset(storage.getItem(LAMP_PRESET_PREFIX+'p2'),'p2').name,'B');
});
