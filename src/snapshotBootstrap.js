/** Register every listener before requesting cached state, so updates during registration are recovered.
 * @param {Promise<unknown>[]} registrations
 * @param {()=>Promise<void>} readSnapshot
 * @param {()=>boolean} active
 */
export async function initializeSnapshot(registrations,readSnapshot,active){await Promise.all(registrations);if(active())await readSnapshot();}
