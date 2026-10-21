import { LOCAL_HOST, store } from "@opensteer/core";
import { useEffect, useRef } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { CSS2DRenderer } from "three/addons/renderers/CSS2DRenderer.js";
import { Labels3, Lines3, Terrain3, Vehicles3 } from "./draw3";

/** Grid cell size in world units; the grid snaps to it while following the focus. */
const CELL = 2;
/** Grid width in world units. */
const GRID_SIZE = 120;
/** Where the camera starts relative to the focus, before scaling to the frame's view_size. */
const START_OFFSET = new THREE.Vector3(0.55, 0.6, 0.6).normalize();

/**
 * Perspective view of the latest 3D frame, with +y up and the ground grid at y = 0, or the
 * demo's terrain when the host sent one (lit, so its slopes show). Drag to
 * orbit, scroll to zoom; the camera follows the frame's focus, keeping the orbit the reader
 * chose. Reads the store in its own render loop, so frames never re-render React.
 */
export function Scene3D() {
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
    const grid = new THREE.GridHelper(GRID_SIZE, GRID_SIZE / CELL, 0x4a4f59, 0x30343c);
    scene.add(grid);
    // Only the terrain is lit; vehicles and lines use unlit materials.
    scene.add(new THREE.HemisphereLight(0xffffff, 0x303030, 1.2));
    const sun = new THREE.DirectionalLight(0xffffff, 1.5);
    sun.position.set(0.5, 1, 0.3);
    scene.add(sun);

    const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 1000);
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = true;
    controls.enablePan = false;

    const vehicles = new Vehicles3(scene);
    const lines = new Lines3(scene);
    const labels = new Labels3(scene);
    const terrain = new Terrain3(scene);
    if (import.meta.env.DEV) Object.assign(window, { __THREE_GAME_DIAGNOSTICS__: { renderer, scene, camera } });

    const resize = () => {
      const { clientWidth: w, clientHeight: h } = el;
      renderer.setSize(w, h, false);
      labelRenderer.setSize(w, h);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
    };
    const observer = new ResizeObserver(resize);
    observer.observe(el);

    // The camera is placed afresh for each demo, then follows the focus by the same offset.
    let placedFor = "";
    const focus = new THREE.Vector3();
    const offset = new THREE.Vector3();
    renderer.setAnimationLoop(() => {
      const host = store.getState().hosts[LOCAL_HOST];
      const frame = host.frame;
      if (frame?.kind === "space") {
        focus.fromArray(frame.focus);
        if (placedFor !== host.demo) {
          placedFor = host.demo;
          controls.target.copy(focus);
          camera.position.copy(focus).addScaledVector(START_OFFSET, frame.view_size);
        } else {
          offset.subVectors(focus, controls.target);
          controls.target.add(offset);
          camera.position.add(offset);
        }
        grid.position.set(Math.round(focus.x / CELL) * CELL, 0, Math.round(focus.z / CELL) * CELL);
        const ground = host.terrain?.demo === host.demo ? host.terrain.view : null;
        grid.visible = !terrain.set(ground);
        vehicles.set(frame.vehicles);
        lines.set(frame.lines, frame.trails);
        labels.set(frame.labels);
      }
      controls.update();
      renderer.render(scene, camera);
      labelRenderer.render(scene, camera);
    });

    return () => {
      renderer.setAnimationLoop(null);
      controls.dispose();
      observer.disconnect();
      vehicles.dispose();
      lines.dispose();
      terrain.dispose();
      grid.dispose();
      renderer.dispose();
      el.replaceChildren();
    };
  }, []);

  return <div ref={mount} className="scene orbit" />;
}
