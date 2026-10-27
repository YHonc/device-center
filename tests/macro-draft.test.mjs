import test from 'node:test';
import assert from 'node:assert/strict';
import {macroKey,parseMacro,validateMacro,appendRecordedAction,importMacroFile,macroKeyOptions,stopRecordingWithKey} from '../src/macroDraft.js';
import {createRequestGate} from '../src/requestGate.js';
const action=(state,code=4)=>({kind:'key',state,code,delayMs:10});
test('focused recorder maps physical keyboard codes and side-specific modifiers',()=>{
  assert.deepEqual(macroKey('KeyA'),{kind:'key',code:4});
  assert.deepEqual(macroKey('Digit0'),{kind:'key',code:39});
  assert.deepEqual(macroKey('ControlRight'),{kind:'modifier',code:228});
  assert.equal(macroKey('Unidentified'),null);
});
test('import keeps only device action fields and rejects stuck keys',()=>{
  assert.deepEqual(parseMacro(JSON.stringify({actions:[{...action('down'),extra:'ignored'},action('up')]})),[action('down'),action('up')]);
  assert.throws(()=>validateMacro([action('down')]),/未抬起/);
  assert.throws(()=>validateMacro([action('up')]),/缺少/);
  assert.throws(()=>validateMacro([action('down'),action('down'),action('up')]),/重复/);
});
test('import rejects invalid delays and capacity overflow before device write',()=>{
  assert.throws(()=>validateMacro([{...action('down'),delayMs:65536},action('up')]),/无效/);
  assert.throws(()=>validateMacro([{...action('down'),delayMs:0},action('up')]),/至少/);
  assert.deepEqual(validateMacro([action('down'),{...action('up'),delayMs:0}]),[action('down'),{...action('up'),delayMs:0}]);
  assert.throws(()=>validateMacro([action('down'),action('up')],1),/最多/);
});
test('recorded event gaps belong to the previous action and preserve chord timing',()=>{
  const actions=[];
  let time=appendRecordedAction(actions,{kind:'modifier',state:'down',code:224},1000,0);
  time=appendRecordedAction(actions,{kind:'key',state:'down',code:4},1040,time);
  time=appendRecordedAction(actions,{kind:'key',state:'up',code:4},1140,time);
  appendRecordedAction(actions,{kind:'modifier',state:'up',code:224},1165,time);
  assert.deepEqual(actions.map(action=>action.delayMs),[40,100,25,0]);
  assert.doesNotThrow(()=>validateMacro(actions));
});
test('late file import is discarded after switching device or macro slot',async()=>{
  for(const scope of ['device','slot']){
    const gate=createRequestGate();let resolve;let draft='new scope';
    const file={text:()=>new Promise(done=>{resolve=done;})};
    const pending=importMacroFile(file,gate.begin()).then(value=>{if(value)draft=value;});
    gate.invalidate();
    resolve(JSON.stringify([action('down'),action('up')]));
    await pending;
    assert.equal(draft,'new scope',scope);
  }
});
test('macro editor offers readable key names for keyboard, modifiers and mouse',()=>{
  assert.equal(macroKeyOptions('key').find(item=>item.code===4).label,'A');
  assert.equal(macroKeyOptions('modifier').find(item=>item.code===228).label,'右 Ctrl');
  assert.equal(macroKeyOptions('mouse').find(item=>item.code===1).label,'左键');
});
test('Escape stops recording without closing the workspace; Tab stops and preserves native focus navigation',()=>{
  for(const code of ['Escape','Tab']){
    const actions=[{kind:'modifier',state:'down',code:224,delayMs:0}];
    let prevented=false,stoppedPropagation=false,recording=true;
    const event={code,preventDefault(){prevented=true;},stopPropagation(){stoppedPropagation=true;}};
    const handled=stopRecordingWithKey(event,()=>{
      appendRecordedAction(actions,{kind:'modifier',state:'up',code:224},100,0);
      recording=false;
    });
    assert.equal(handled,true);
    assert.equal(recording,false);
    assert.equal(prevented,code==='Escape');
    assert.equal(stoppedPropagation,code==='Escape');
    assert.doesNotThrow(()=>validateMacro(actions));
    assert.ok(macroKeyOptions('key').some(option=>option.code===macroKey(code).code),'control key remains manually editable');
  }
  assert.equal(stopRecordingWithKey({code:'KeyA',preventDefault(){assert.fail();},stopPropagation(){assert.fail();}},()=>assert.fail()),false);
});
