import test from 'node:test';
import assert from 'node:assert/strict';
import {filterMappingOptions} from '../src/mappingSearch.js';
import {lightingChanges} from '../src/lightingDraft.js';

test('mapping search filters both named actions and ATK keyboard options with readable empty results',()=>{
  const actions=[{id:'media:play',label:'播放 / 暂停'},{id:'mouse:left',label:'左键'}];
  const keys=[{value:4,label:'A'},{value:5,label:'B'},{value:40,label:'Enter'}];
  assert.deepEqual(filterMappingOptions(actions,'播放'),[actions[0]]);
  assert.deepEqual(filterMappingOptions(keys,'  enter  '),[keys[2]]);
  assert.deepEqual(filterMappingOptions(keys,'not a key'),[]);
  assert.deepEqual(filterMappingOptions(actions,'   '),actions);
});

test('switching lighting mode sends only mode and preserves the unknown target parameter slot',()=>{
  const snapshot={mode:1,brightness:3,speed:2,multiColor:false,rgb:[255,0,0]};
  const selected={brightnessEditable:true,speedEditable:true,colorEditable:true};
  assert.deepEqual(lightingChanges(snapshot,{mode:2,brightness:1,speed:4,multiColor:true,rgb:[0,0,255]},selected),{mode:2});
  const readback={mode:2,brightness:4,speed:1,multiColor:false,rgb:[10,20,30]};
  assert.deepEqual(lightingChanges(readback,{...readback,brightness:2},selected),{brightness:2});
  assert.deepEqual(lightingChanges(readback,readback,selected),{});
  assert.deepEqual(lightingChanges(null,readback,selected),{});
});
