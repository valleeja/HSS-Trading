// 🧩 Bac à sable des modules d'Argos HSS Trading dans le navigateur, interface v2 du kit module Argos 1.0.0
// (INTERFACE-V2.md) : un Worker par appel, arrêté par la page s'il dépasse son temps. L'entrée arrive déjà au format
// du kit (faite par l'app) et la sortie repart telle quelle (vérifiée par l'app avec le décodeur du kit).
// Les fonctions d'hôte du kit (module « argos ») répondent toutes « refusé » (-3) ici : ni réseau, ni fichiers, ni
// IA, jamais de clé.
const DENIED = -3;
const MAX = 4 * 1024 * 1024;
onmessage = async (e) => {
  const { mod, input } = e.data;
  try {
    const deny = () => DENIED;
    const host = { read_file: deny, write_file: deny, network_request: deny, ai_request: deny, model_request: deny };
    const inst = await WebAssembly.instantiate(mod, { argos: host });
    const x = inst.exports, mem = x.memory;
    if (typeof x.run === "function") throw new Error("un module v2 n'exporte pas « run »");
    if (x.argos_abi() !== 2) throw new Error("interface différente (v2 attendue)");
    if (input.length > MAX) throw new Error("entrée trop grande (4 Mio au plus)");
    const p = x.alloc(input.length);
    if (p < 0 || p + input.length > mem.buffer.byteLength) throw new Error("mémoire d'entrée refusée");
    new Uint8Array(mem.buffer, p, input.length).set(input);
    const r = BigInt(x.run_data(p, input.length));
    if (r < 0n) throw new Error("le module a renvoyé l'erreur " + r);
    const op = Number(r >> 32n), on = Number(r & 0xffffffffn);
    if (on > MAX || op + on > mem.buffer.byteLength) throw new Error("sortie hors limites");
    postMessage({ ok: true, out: new Uint8Array(mem.buffer, op, on).slice() });
  } catch (err) {
    postMessage({ ok: false, err: String((err && err.message) || err) });
  }
};
