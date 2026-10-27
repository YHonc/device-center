import test from 'node:test';import assert from 'node:assert/strict';import{compactCloseAction,finishCompactClose}from'../src/windowCloseRoute.js';
test('快捷窗忽略应用退出广播，主窗确认不会被快捷窗绕过',()=>{
  const dispatched=[];
  const compactListener=payload=>{const action=compactCloseAction(payload);if(action)dispatched.push(action);};
  compactListener({exit:true});compactListener({});compactListener(null);
  assert.deepEqual(dispatched,[]);
  compactListener({exit:false});
  assert.deepEqual(dispatched,['close_device_window']);
  assert.equal(dispatched.includes('exit_device_center'),false);
});
test('快捷窗定向退出先等待待发送/正在写入队列，再允许退出',async()=>{let resolve;const order=[];const pending=new Promise(r=>resolve=r);const done=finishCompactClose(true,async()=>{order.push('flush');await pending;order.push('confirmed');},()=>false,async()=>order.push('exit'));await Promise.resolve();assert.deepEqual(order,['flush']);resolve();assert.equal(await done,true);assert.deepEqual(order,['flush','confirmed','exit']);});
test('快捷窗写入失败不会退出，保留窗口供用户处理',async()=>{let exits=0;assert.equal(await finishCompactClose(true,async()=>{},()=>true,async()=>exits++),false);assert.equal(exits,0);});
test('关闭等待写入期间收到定向退出，完成后保留退出意图',async()=>{let resolve;let exit=false;const actions=[];const pending=new Promise(r=>resolve=r);const done=finishCompactClose(true,async()=>{await pending;},()=>false,async()=>actions.push(exit?'exit':'close'));exit=true;resolve();await done;assert.deepEqual(actions,['exit']);});
