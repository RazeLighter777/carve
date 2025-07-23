<template>
  <div class="relative w-full h-full">
    <canvas id="flameCanvas" ref="canvasRef"></canvas>
    <div class="absolute bottom-0 left-0 w-full flex justify-center mb-2">
      <span style="font-size: 2rem;">🎃</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';

interface Particle {
  x: number;
  y: number;
  size: number;
  speedX: number;
  speedY: number;
  life: number;
  maxLife: number;
}

const canvasRef = ref<HTMLCanvasElement | null>(null);
let ctx: CanvasRenderingContext2D;
let width: number;
let height: number;
const particles: Particle[] = [];
let animationFrameId: number;

function resizeCanvas(): void {
  if (!canvasRef.value) return;
  const parent = canvasRef.value.parentElement;
  if (parent) {
    width = canvasRef.value.width = parent.offsetWidth;
    height = canvasRef.value.height = parent.offsetHeight;
  }
}

function createParticle(): void {
  const x = width / 2 + (Math.random() - 0.5) * 20;
  const y = height - 60;
  const size = Math.random() * 8 + 6;
  const speedY = Math.random() * -2.2 - 1;
  const speedX = (Math.random() - 0.5) * 0.7;
  const life = 80 + Math.random() * 40;
  particles.push({ x, y, size, speedX, speedY, life, maxLife: life });
}

function drawParticle(p: Particle): void {
  const alpha = p.life / p.maxLife;
  const r = 255;
  const g = Math.floor(120 + 100 * (1 - alpha));
  const b = 0;
  const gradient = ctx.createRadialGradient(p.x, p.y, 0, p.x, p.y, p.size);
  gradient.addColorStop(0, `rgba(${r}, ${g}, ${b}, ${alpha})`);
  gradient.addColorStop(0.4, `rgba(${r}, ${g}, ${b}, ${alpha * 0.7})`);
  gradient.addColorStop(1, `rgba(0, 0, 0, 0)`);
  ctx.fillStyle = gradient;
  ctx.beginPath();
  ctx.arc(p.x, p.y, p.size, 0, Math.PI * 2);
  ctx.fill();
}

function animate(): void {
  ctx.clearRect(0, 0, width, height);
  for (let i = particles.length - 1; i >= 0; i--) {
    const p = particles[i];
    p.x += p.speedX;
    p.y += p.speedY;
    p.life--;
    drawParticle(p);
    if (p.life <= 0) {
      particles.splice(i, 1);
    }
  }
  for (let i = 0; i < 6; i++) {
    createParticle();
  }
  animationFrameId = requestAnimationFrame(animate);
}

onMounted(() => {
  if (!canvasRef.value) return;
  ctx = canvasRef.value.getContext('2d')!;
  resizeCanvas();
  window.addEventListener('resize', resizeCanvas);
  animate();
});

onUnmounted(() => {
  window.removeEventListener('resize', resizeCanvas);
  cancelAnimationFrame(animationFrameId);
});
</script>

<style scoped>
canvas {
  display: block;
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  z-index: 0;
}
</style>