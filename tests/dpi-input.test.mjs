import test from 'node:test';import assert from 'node:assert/strict';import {snapDpi,dpiKeyboardValue,isDpiAdjustmentKey} from '../src/dpiInput.js';
test('slider uses absolute sensor step instead of the range input min offset',()=>{
  const capability={min:50,max:42000,stepSegments:[{until:30000,step:50},{until:42000,step:100}]};
  assert.equal(snapDpi(30150,capability),30200);
  assert.equal(snapDpi(30020,capability),30000);
  assert.equal(snapDpi(29974,capability),29950);
  assert.equal(snapDpi(-1,capability),50);
});
test('keyboard arrows cross sensor step boundaries in both directions',()=>{
  const capability={min:10,max:42000,stepSegments:[{until:10000,step:10},{until:30000,step:50},{until:42000,step:100}]};
  for(const [start,next] of [[10,20],[9990,10000],[10000,10050],[29950,30000],[30000,30100],[41900,42000]]){
    assert.equal(dpiKeyboardValue('ArrowRight',start,capability),next);
    assert.equal(dpiKeyboardValue('ArrowUp',start,capability),next);
    assert.equal(dpiKeyboardValue('ArrowLeft',next,capability),start);
    assert.equal(dpiKeyboardValue('ArrowDown',next,capability),start);
  }
  assert.equal(dpiKeyboardValue('ArrowDown',10,capability),10);
  assert.equal(dpiKeyboardValue('ArrowUp',42000,capability),42000);
  assert.equal(dpiKeyboardValue('Home',30000,capability),10);
  assert.equal(dpiKeyboardValue('End',30000,capability),42000);
  assert.equal(dpiKeyboardValue('PageUp',30000,capability),31000);
  assert.equal(dpiKeyboardValue('PageDown',30000,capability),29500);
  assert.equal(dpiKeyboardValue('Tab',30000,capability),null);
});
test('keyboard adjustment recovers a typed off-step value without becoming stuck',()=>{
  const capability={min:50,max:36000,stepSegments:[{until:30000,step:50},{until:36000,step:100}]};
  assert.equal(dpiKeyboardValue('ArrowUp',30050,capability),30100);
  assert.equal(dpiKeyboardValue('ArrowDown',30050,capability),30000);
});
test('DPI number editor preserves text navigation and modified shortcuts',()=>{
  for(const key of ['ArrowLeft','ArrowRight','Home','End']){
    assert.equal(isDpiAdjustmentKey({key},'number'),false,key);
    assert.equal(isDpiAdjustmentKey({key},'range'),true,key);
  }
  for(const key of ['ArrowUp','ArrowDown','PageUp','PageDown']){
    assert.equal(isDpiAdjustmentKey({key},'number'),true,key);
    for(const modifier of ['altKey','ctrlKey','metaKey','shiftKey']){
      assert.equal(isDpiAdjustmentKey({key,[modifier]:true},'number'),false,`${modifier}+${key}`);
      assert.equal(isDpiAdjustmentKey({key,[modifier]:true},'range'),false,`${modifier}+${key}`);
    }
  }
  assert.equal(isDpiAdjustmentKey({key:'Tab'},'number'),false);
});
