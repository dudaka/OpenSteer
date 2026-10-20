import { LOCAL_HOST, store } from "@opensteer/core";
import { useEffect, useRef } from "react";
import * as THREE from "three";
import { CSS2DRenderer } from "three/addons/renderers/CSS2DRenderer.js";
import { Labels, Lines, Vehicles } from "./draw";

/** World units visible across the shorter side of the view. */
const VIEW_SIZE = 20;
/** Grid cell size in world units; the grid snaps to it while following the focus. */
const CELL = 2;

/**
 * Top-down orthographic view of the latest frame, centred on its focus. A world point (x, y)
 * is drawn at (x, 0, y): +x to the right, +y down the screen. Reads the store in its own
 * render loop, so frames never re-render React.
 */
export function Scene() {
  const mount = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = mount.current!;
    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(devicePixelRatio);
    el.appendChild(renderer.domElement);
    const labelRenderer = new CSS2DRenderer();
    labelRenderer.domElement.className = "labels";
    el.appendChild(labelRenderer.domElement);

    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x1d1f24);
    const grid = new THREE.GridHelper(VIEW_SIZE * 3, (VIEW_SIZE * 3) / CELL, 0x4a4f59, 0x30343c);
    grid.position.y = -0.01;
    scene.add(grid);

    const camera = new THREE.OrthographicCamera();
    camera.up.set(0, 0, -1);

    const vehicles = new Vehicles(scene);
    const lines = new Lines(scene);
    const labels = new Labels(scene);
    if (import.meta.env.DEV) Object.assign(window, { __THREE_GAME_DIAGNOSTICS__: { renderer, scene, camera } });

    const resize = () => {
      const { clientWidth: w, clientHeight: h } = el;
      renderer.setSize(w, h, false);
      labelRenderer.setSize(w, h);
      const half = VIEW_SIZE / 2;
      const [x, y] = w >= h ? [half * (w / h), half] : [half, half * (h / w)];
      Object.assign(camera, { left: -x, right: x, top: y, bottom: -y, near: 0.1, far: 100 });
      camera.updateProjectionMatrix();
    };
    const observer = new ResizeObserver(resize);
    observer.observe(el);

    renderer.setAnimationLoop(() => {
      const frame = store.getState().hosts[LOCAL_HOST].frame;
      if (frame) {
        const [fx, fy] = frame.focus;
        camera.position.set(fx, 50, fy);
        camera.lookAt(fx, 0, fy);
        grid.position.set(Math.round(fx / CELL) * CELL, -0.01, Math.round(fy / CELL) * CELL);
        vehicles.set(frame.vehicles);
        lines.set(frame.lines, frame.trails);
        labels.set(frame.labels);
      }
      renderer.render(scene, camera);
      labelRenderer.render(scene, camera);
    });

    return () => {
      renderer.setAnimationLoop(null);
      observer.disconnect();
      vehicles.dispose();
      lines.dispose();
      grid.dispose();
      renderer.dispose();
      el.replaceChildren();
    };
  }, []);

  return <div ref={mount} className="scene" />;
}
