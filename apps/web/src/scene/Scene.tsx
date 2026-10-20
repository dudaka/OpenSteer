import { LOCAL_HOST, store } from "@opensteer/core";
import { useEffect, useRef } from "react";
import * as THREE from "three";
import { CSS2DRenderer } from "three/addons/renderers/CSS2DRenderer.js";
import { useHost } from "../host";
import { Labels, Lines, Vehicles } from "./draw";

/** Grid cell size in world units; the grid snaps to it while following the focus. */
const CELL = 2;
/** Grid width in world units: wider than the largest view a demo asks for. */
const GRID_SIZE = 240;

/**
 * Top-down orthographic view of the latest frame, centred on its focus and showing its
 * view_size world units across the shorter side. A world point (x, y)
 * is drawn at (x, 0, y): +x to the right, +y down the screen. Reads the store in its own
 * render loop, so frames never re-render React.
 */
export function Scene() {
  const mount = useRef<HTMLDivElement>(null);
  const host = useHost();

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
    const grid = new THREE.GridHelper(GRID_SIZE, GRID_SIZE / CELL, 0x4a4f59, 0x30343c);
    grid.position.y = -0.01;
    scene.add(grid);

    const camera = new THREE.OrthographicCamera();
    camera.up.set(0, 0, -1);

    const vehicles = new Vehicles(scene);
    const lines = new Lines(scene);
    const labels = new Labels(scene);
    if (import.meta.env.DEV) Object.assign(window, { __THREE_GAME_DIAGNOSTICS__: { renderer, scene, camera } });

    let viewSize = 20;
    const resize = () => {
      const { clientWidth: w, clientHeight: h } = el;
      renderer.setSize(w, h, false);
      labelRenderer.setSize(w, h);
      const half = viewSize / 2;
      const [x, y] = w >= h ? [half * (w / h), half] : [half, half * (h / w)];
      Object.assign(camera, { left: -x, right: x, top: y, bottom: -y, near: 0.1, far: 100 });
      camera.updateProjectionMatrix();
    };
    const observer = new ResizeObserver(resize);
    observer.observe(el);

    // Input handling: a click becomes the world point it hits on the ground plane.
    const ground = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);
    const raycaster = new THREE.Raycaster();
    const hit = new THREE.Vector3();
    const onClick = (e: PointerEvent) => {
      const rect = renderer.domElement.getBoundingClientRect();
      const ndc = new THREE.Vector2(((e.clientX - rect.left) / rect.width) * 2 - 1, -((e.clientY - rect.top) / rect.height) * 2 + 1);
      raycaster.setFromCamera(ndc, camera);
      if (raycaster.ray.intersectPlane(ground, hit)) void host.click(hit.x, hit.z);
    };
    renderer.domElement.addEventListener("pointerdown", onClick);

    renderer.setAnimationLoop(() => {
      const frame = store.getState().hosts[LOCAL_HOST].frame;
      if (frame) {
        const [fx, fy] = frame.focus;
        if (frame.view_size !== viewSize) {
          viewSize = frame.view_size;
          resize();
        }
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
      renderer.domElement.removeEventListener("pointerdown", onClick);
      observer.disconnect();
      vehicles.dispose();
      lines.dispose();
      grid.dispose();
      renderer.dispose();
      el.replaceChildren();
    };
  }, [host]);

  return <div ref={mount} className="scene" />;
}
