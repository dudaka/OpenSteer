import { LOCAL_HOST, store } from "@opensteer/core";
import { useEffect, useRef } from "react";
import * as THREE from "three";

/** World units visible across the shorter side of the view. */
const VIEW_SIZE = 40;

/**
 * Top-down orthographic view of the latest frame. A world point (x, y) is drawn at
 * (x, 0, y): +x to the right, +y down the screen. Reads the store in its own render loop,
 * so frames never re-render React.
 */
export function Scene() {
  const mount = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = mount.current!;
    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(devicePixelRatio);
    el.appendChild(renderer.domElement);

    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x1d1f24);
    scene.add(new THREE.GridHelper(VIEW_SIZE, VIEW_SIZE / 2, 0x4a4f59, 0x30343c));

    const camera = new THREE.OrthographicCamera();
    camera.position.set(0, 50, 0);
    camera.up.set(0, 0, -1);
    camera.lookAt(0, 0, 0);

    const disc = new THREE.CircleGeometry(1, 32).rotateX(-Math.PI / 2);
    const material = new THREE.MeshBasicMaterial({ color: 0xf2b134 });
    const meshes: THREE.Mesh[] = [];

    const resize = () => {
      const { clientWidth: w, clientHeight: h } = el;
      renderer.setSize(w, h, false);
      const half = VIEW_SIZE / 2;
      const [x, y] = w >= h ? [half * (w / h), half] : [half, half * (h / w)];
      Object.assign(camera, { left: -x, right: x, top: y, bottom: -y, near: 0.1, far: 100 });
      camera.updateProjectionMatrix();
    };
    const observer = new ResizeObserver(resize);
    observer.observe(el);

    renderer.setAnimationLoop(() => {
      const vehicles = store.getState().hosts[LOCAL_HOST].frame?.vehicles ?? [];
      while (meshes.length < vehicles.length) {
        const mesh = new THREE.Mesh(disc, material);
        scene.add(mesh);
        meshes.push(mesh);
      }
      meshes.forEach((mesh, i) => {
        const v = vehicles[i];
        mesh.visible = v !== undefined;
        if (v) {
          mesh.position.set(v.x, 0, v.y);
          mesh.scale.setScalar(v.radius);
        }
      });
      renderer.render(scene, camera);
    });

    return () => {
      renderer.setAnimationLoop(null);
      observer.disconnect();
      disc.dispose();
      material.dispose();
      renderer.dispose();
      el.removeChild(renderer.domElement);
    };
  }, []);

  return <div ref={mount} className="scene" />;
}
