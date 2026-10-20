// Pooled three.js objects for a FrameView. World [x, y] is drawn at (x, 0, y). Nothing here
// allocates per frame once the pools have grown.
import type { LabelView, LineView, TrailView, VehicleView } from "@opensteer/core";
import * as THREE from "three";
import { CSS2DObject } from "three/addons/renderers/CSS2DRenderer.js";

const color = new THREE.Color();

/** OpenSteer's basic 2D body (`drawBasic2dCircularVehicle`) at radius 1, facing +z. */
function bodyGeometry(): THREE.BufferGeometry {
  const back = -Math.sqrt(1 - 0.5 * 0.5);
  return new THREE.BufferGeometry().setFromPoints([
    new THREE.Vector3(0, 0, 1),
    new THREE.Vector3(-0.5, 0, back),
    new THREE.Vector3(0.5, 0, back),
  ]);
}

function boundaryGeometry(): THREE.BufferGeometry {
  const points = Array.from({ length: 20 }, (_, i) => {
    const a = (i / 20) * Math.PI * 2;
    return new THREE.Vector3(Math.sin(a), 0, Math.cos(a));
  });
  return new THREE.BufferGeometry().setFromPoints(points);
}

export class Vehicles {
  private readonly body = bodyGeometry();
  private readonly boundary = boundaryGeometry();
  private readonly boundaryMaterial = new THREE.LineBasicMaterial({ color: 0xffffff });
  private readonly pool: { group: THREE.Group; material: THREE.MeshBasicMaterial }[] = [];

  constructor(private readonly scene: THREE.Scene) {}

  set(vehicles: VehicleView[]): void {
    while (this.pool.length < vehicles.length) {
      const material = new THREE.MeshBasicMaterial({ side: THREE.DoubleSide });
      const group = new THREE.Group();
      group.add(new THREE.Mesh(this.body, material), new THREE.LineLoop(this.boundary, this.boundaryMaterial));
      this.scene.add(group);
      this.pool.push({ group, material });
    }
    this.pool.forEach(({ group, material }, i) => {
      const v = vehicles[i];
      group.visible = v !== undefined;
      if (!v) return;
      group.position.set(v.position[0], 0, v.position[1]);
      group.rotation.y = Math.atan2(v.forward[0], v.forward[1]);
      group.scale.setScalar(v.radius);
      material.color.setHex(v.color);
    });
  }

  dispose(): void {
    this.body.dispose();
    this.boundary.dispose();
    this.boundaryMaterial.dispose();
    for (const { material } of this.pool) material.dispose();
  }
}

/** Annotation lines and trails as one LineSegments with per-vertex colors. */
export class Lines {
  private readonly geometry = new THREE.BufferGeometry();
  private readonly material = new THREE.LineBasicMaterial({ vertexColors: true });
  private positions = new Float32Array(0);
  private colors = new Float32Array(0);
  private count = 0;

  constructor(scene: THREE.Scene) {
    const segments = new THREE.LineSegments(this.geometry, this.material);
    // The points change every frame, so a bounding sphere computed once goes stale and the
    // lines get culled as soon as the camera follows the vehicle away from it.
    segments.frustumCulled = false;
    scene.add(segments);
  }

  set(lines: LineView[], trails: TrailView[]): void {
    const segments = lines.length + trails.reduce((n, t) => n + Math.max(t.points.length - 1, 0), 0);
    this.reserve(segments);
    this.count = 0;
    for (const l of lines) this.push(l.from, l.to, l.color);
    for (const t of trails) {
      for (let i = 1; i < t.points.length; i++) this.push(t.points[i - 1], t.points[i], t.color);
    }
    this.geometry.setDrawRange(0, this.count * 2);
    this.geometry.attributes.position.needsUpdate = true;
    this.geometry.attributes.color.needsUpdate = true;
  }

  dispose(): void {
    this.geometry.dispose();
    this.material.dispose();
  }

  /** Grows the buffers (doubling) when a frame has more segments than ever before. */
  private reserve(segments: number): void {
    if (segments * 6 <= this.positions.length) return;
    const capacity = Math.max(64, segments * 2) * 6;
    this.positions = new Float32Array(capacity);
    this.colors = new Float32Array(capacity);
    this.geometry.setAttribute("position", new THREE.BufferAttribute(this.positions, 3));
    this.geometry.setAttribute("color", new THREE.BufferAttribute(this.colors, 3));
  }

  private push(from: [number, number], to: [number, number], hex: number): void {
    color.setHex(hex);
    const i = this.count * 6;
    this.positions.set([from[0], 0, from[1], to[0], 0, to[1]], i);
    this.colors.set([color.r, color.g, color.b, color.r, color.g, color.b], i);
    this.count++;
  }
}

/** Text labels as HTML elements over the canvas (CSS2DRenderer). */
export class Labels {
  private readonly pool: CSS2DObject[] = [];

  constructor(private readonly scene: THREE.Scene) {}

  set(labels: LabelView[]): void {
    while (this.pool.length < labels.length) {
      const element = document.createElement("div");
      element.className = "label";
      const label = new CSS2DObject(element);
      // Anchor the bottom-left corner at the point, so text sits beside it, not on it.
      label.center.set(0, 1);
      this.scene.add(label);
      this.pool.push(label);
    }
    this.pool.forEach((label, i) => {
      const l = labels[i];
      label.visible = l !== undefined;
      if (!l) return;
      label.position.set(l.at[0], 0, l.at[1]);
      label.element.textContent = l.text;
      label.element.style.color = `#${l.color.toString(16).padStart(6, "0")}`;
    });
  }
}
