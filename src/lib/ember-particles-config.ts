import type { ISourceOptions } from "@tsparticles/engine";

const EMBER_SPAWN_X = [4, 11, 18, 26, 34, 42, 50, 58, 66, 74, 82, 91];

const DARK_EMBER_COLORS = [
  "#ff6b1a",
  "#ff5722",
  "#f4511e",
  "#e64a19",
  "#d84315",
  "#c62828",
  "#b71c1c",
  "#8b2500",
];

const LIGHT_EMBER_COLORS = [
  "#ea580c",
  "#f97316",
  "#dc2626",
  "#c2410c",
  "#b91c1c",
  "#9a3412",
  "#7f1d1d",
];

function emberPaint(palette: string[]) {
  return {
    fill: {
      enable: true,
      color: { value: palette },
    },
  };
}

function buildEmitter(x: number, index: number, isDark: boolean) {
  const palette = isDark ? DARK_EMBER_COLORS : LIGHT_EMBER_COLORS;

  return {
    autoPlay: true,
    fill: true,
    position: { x, y: 100 },
    size: { width: isDark ? 6 : 5, height: 0 },
    rate: {
      delay: 0.28 + (index % 6) * 0.09,
      quantity: index % 4 === 0 ? 2 : 1,
    },
    life: { count: 0, duration: 0.1, wait: false },
    particles: {
      paint: emberPaint(palette),
      shape: { type: "circle" },
      opacity: {
        value: { min: isDark ? 0.5 : 0.6, max: 1 },
        animation: {
          enable: true,
          speed: 1,
          sync: false,
          destroy: "min",
        },
      },
      size: {
        value: { min: 0.8, max: isDark ? 3 : 2.4 },
        animation: {
          enable: true,
          speed: 3,
          sync: false,
          destroy: "min",
        },
      },
      move: {
        enable: true,
        speed: { min: 0.2, max: isDark ? 1.2 : 0.8 },
        direction: "top",
        random: true,
        straight: false,
        outModes: { default: "destroy" },
        gravity: {
          enable: true,
          acceleration: isDark ? -1.5 : -1,
          maxSpeed: 1.5,
        },
        drift: { min: -1.5, max: 1.5 },
      },
      life: {
        duration: { value: { min: 2, max: isDark ? 5 : 7 } },
        count: 1,
      },
    },
  };
}

export function getEmberParticleOptions(isDark: boolean): ISourceOptions {
  const spawnPoints = EMBER_SPAWN_X;

  return {
    fullScreen: { enable: false },
    fpsLimit: 60,
    detectRetina: true,
    background: { color: { value: "transparent" } },
    particles: {
      number: { value: 0 },
    },
    interactivity: {
      detectsOn: "window",
      events: {
        onHover: { enable: false },
        onClick: { enable: false },
        resize: { enable: true },
      },
    },
    emitters: spawnPoints.map((x, i) => buildEmitter(x, i, isDark)),
  };
}
