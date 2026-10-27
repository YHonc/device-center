/** @typedef {import('./lampPresets.js').LampPreset} LampPreset */
/** Compare the item originally opened for editing with the latest stored item.
 * @param {LampPreset|null|undefined} current @param {LampPreset|null|undefined} expected */
export function sameLampPreset(current,expected){
  return !!current&&!!expected&&current.id===expected.id&&current.name===expected.name&&current.brightness===expected.brightness&&current.temperature===expected.temperature;
}
