import test from 'node:test';
import assert from 'node:assert/strict';
import {selectAutomaticLamp,isCurrentAttempt} from '../src/cloudSelection.js';
const lamp=(id,ip='192.168.1.2')=>({id,ip,model:'yeelink.light.lamp22',eligible:true});
test('唯一候选自动选中；不接受不支持型号或不可导入项',()=>{
  assert.equal(selectAutomaticLamp([lamp('one')],null,null).deviceId,'one');
  assert.equal(selectAutomaticLamp([{...lamp('bad'),eligible:false},{...lamp('other'),model:'unknown'}],null,null).reason,'none');
});
test('多台不按数组顺序选择；保存目标唯一时优先',()=>{
  const devices=[lamp('a'),lamp('b','192.168.1.3')];
  assert.equal(selectAutomaticLamp(devices,null,null).reason,'ambiguous');
  assert.equal(selectAutomaticLamp([...devices].reverse(),null,null).deviceId,null);
  assert.equal(selectAutomaticLamp(devices,'192.168.1.3','192.168.1.2').deviceId,'b');
  assert.equal(selectAutomaticLamp([lamp('a'),lamp('dup')],'192.168.1.2',null).reason,'ambiguous');
});
test('已保存或输入的明确目标失配时，不误选唯一另一台设备',()=>{
  assert.equal(selectAutomaticLamp([lamp('a')],'192.168.1.9',null).reason,'missing-target');
  assert.equal(selectAutomaticLamp([lamp('a')],'192.168.1.9','192.168.1.8').reason,'missing-target');
  assert.equal(selectAutomaticLamp([lamp('a')],null,'192.168.1.2').reason,'expected');
  assert.equal(selectAutomaticLamp([],null,null).reason,'none');
});
test('取消后迟到列表或导入结果不会触发后续回调',async()=>{
  let generation=1;const captured=generation;let resolve;const pending=new Promise(r=>{resolve=r;});let callbacks=0;
  const task=pending.then(()=>{if(isCurrentAttempt(captured,generation,false))callbacks++;});
  generation++;resolve();await task;assert.equal(callbacks,0);
  assert.equal(isCurrentAttempt(generation,generation,true),false);
  assert.equal(isCurrentAttempt(generation,generation,false),true);
});
