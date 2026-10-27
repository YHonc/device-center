import test from 'node:test';import assert from 'node:assert/strict';
import {performanceFields,settingValueLabel} from '../src/settingsPresentation.js';

test('KX performance excludes mouse DPI and lighting fields while retaining keyboard controls',()=>{
  const fields=['reportRateHz','dpiLevel','lightingBrightness','sleepSeconds','lowLatency','macMode','winLock'].map(key=>({key}));
  assert.deepEqual(performanceFields(fields,true).map(field=>field.key),['reportRateHz','sleepSeconds','lowLatency','macMode','winLock']);
  assert.deepEqual(performanceFields(fields,false).map(field=>field.key),['reportRateHz','dpiLevel']);
});
test('confirmed numeric toggle readings use their user-visible option labels',()=>{
  const field={key:'macMode',options:[{label:'关闭',value:0},{label:'开启',value:1}]};
  assert.equal(settingValueLabel(field,0),'关闭');assert.equal(settingValueLabel(field,1),'开启');
  assert.equal(settingValueLabel({key:'reportRateHz',options:[{label:'1000 Hz',value:1000}]},1000),'1000 Hz');
  assert.equal(settingValueLabel(field,null),'未知');assert.equal(settingValueLabel({key:'sleepSeconds'},0),'永不休眠');
});
