/** Track asynchronous event registrations for one WebView. */
export function createListenerScope(){
  let closed=false;
  /** @type {Set<()=>void>} */ const stops=new Set();
  return{
    /** @param {()=>void} stop */
    add(stop){if(closed)stop();else stops.add(stop);},
    close(){if(closed)return;closed=true;for(const stop of stops)stop();stops.clear();}
  };
}
