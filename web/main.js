import init, { App, AppWindow, TilingGenerator as Tiling } from './hyperbolic.js';

let app, tilingGenerator;
let container = document.getElementById('outer-view');
let depth = document.getElementById('depth');
view.width = container.clientWidth;
view.height = container.clientHeight;

addEventListener('resize', e => {
  let width = container.clientWidth,
      height = container.clientHeight;
  view.width = width;
  view.height = height;
  app.resize(width, height);
});
view.addEventListener('pointermove', e => {
  if(e.buttons & 1 != 0) {
  	e.target.setPointerCapture(e.pointerId);
    app.update_delta(e.clientX, e.clientY);
  }
});
view.addEventListener('pointerup', e => app.reset_delta());

for(let p of document.getElementsByClassName('projection')) {
  p.addEventListener('input', e => {
    app.set_projection(e.target.value);
  });
}
depth.addEventListener('input', e => app.set_depth(Number(e.target.value)));

schlafliP.addEventListener('input', e => {
  schlafliQ.min = Math.floor(3 + 4 / (+e.target.value - 2));
  schlafliQ.max = +schlafliQ.min + 10;
});
submitTiling.addEventListener('click', e => {
  e.preventDefault();
  tilingGenerator = new Tiling(+schlafliP.value, +schlafliQ.value, tiling.value);
  app.set_tiling(tilingGenerator, Number(depth.value));
});

async function run() {
  await init();
  tilingGenerator = new Tiling(+schlafliP.value, +schlafliQ.value, tiling.value);
  let window = new AppWindow(document.getElementById('view'), () => requestAnimationFrame(() => app.draw()));
  app = await new App(tilingGenerator, window);
  app.set_depth(Number(depth.value));
}
run();
