import type { Engine } from "@tsparticles/engine";
import { loadEmittersPlugin } from "@tsparticles/plugin-emitters";
import { loadSlim } from "@tsparticles/slim";

export async function initParticlesEngine(engine: Engine): Promise<void> {
  await loadSlim(engine);
  await loadEmittersPlugin(engine);
}
