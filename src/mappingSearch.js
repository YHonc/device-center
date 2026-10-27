/** @template {{label:string}} T @param {T[]} options @param {string} query @returns {T[]} */
export function filterMappingOptions(options,query){
  const search=query.trim().toLocaleLowerCase();
  return search?options.filter(option=>option.label.toLocaleLowerCase().includes(search)):options;
}
