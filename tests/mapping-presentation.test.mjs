import test from 'node:test';
import assert from 'node:assert/strict';
import {keycapLabel} from '../src/mappingPresentation.js';

test('Home physical slot displays a read Print mapping while preserving its apply identity',()=>{
  const button={id:'Home',currentAction:'key:Print'};
  assert.equal(keycapLabel(button.id,button.currentAction,id=>id),'Print');
  assert.equal(button.id,'Home');
});
test('unread or non-key mappings retain readable physical key labels',()=>{
  const label=id=>({one:'1 !',backquote:'` ~'}[id]??id);
  assert.equal(keycapLabel('one',null,label),'1 !');
  assert.equal(keycapLabel('backquote','taskManager',label),'` ~');
});
