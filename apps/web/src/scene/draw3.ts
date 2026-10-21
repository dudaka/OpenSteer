// Pooled three.js objects for a Frame3View. World [x, y, z] (+y up) is drawn as it is. Nothing
// here allocates per frame once the pools have grown.
import type { Label3View, Line3View, TerrainView, Trail3View, Vehicle3View } from "@opensteer/core";
import * as THREE from "three";
import { CSS2DObject } from "three/addons/renderers/CSS2DRenderer.js";

const color = new THREE.Color();
const forward = new THREE.Vector3();
const up = new THREE.Vector3();
const side = new THREE.Vector3();
const basis = new THREE.Matrix4();

/** OpenSteer's basic body at radius 1: a flat triangle in the local XZ plane, facing +z. */
function bodyGeometry(): THREE.BufferGeometry {
  const back = -Math.sqrt(1 - 0.5 * 0.5);
  return new THREE.BufferGeometry().setFromPoints([
    new THREE.Vector3(0, 0, 1),
    new THREE.Vector3(-0.5, 0, back),
    new THREE.Vector3(0.5, 0, back),
  ]);
}

/** A short mast along local +y, so the vehicle's up (and its roll) is visible. */
function mastGeometry(): THREE.BufferGeometry {
  return new THREE.BufferGeometry().setFromPoints([new THREE.Vector3(0, 0, 0), new THREE.Vector3(0, 0.6, 0)]);
}

export class Vehicles3 {
  private readonly body = bodyGeometry();
  private readonly mast = mastGeometry();
  private readonly mastMaterial = new THREE.LineBasicMaterial({ color: 0xffffff });
  private readonly pool: { group: THREE.Group; material: THREE.MeshBasicMaterial }[] = [];

  constructor(private readonly scene: THREE.Scene) {}

  set(vehicles: Vehicle3View[]): void {
    while (this.pool.length < vehicles.length) {
      const material = new THREE.MeshBasicMaterial({ side: THREE.DoubleSide });
      const group = new THREE.Group();
      group.matrixAutoUpdate = false;
      group.add(new THREE.Mesh(this.body, material), new THREE.LineSegments(this.mast, this.mastMaterial));
      this.scene.add(group);
      this.pool.push({ group, material });
    }
    this.pool.forEach(({ group, material }, i) => {
      const v = vehicles[i];
      group.visible = v !== undefined;
      if (!v) return;
      // Local +z is the vehicle's forward and local +y its up; local +x = up x forward.
      forward.fromArray(v.forward);
      up.fromArray(v.up);
      side.crossVectors(up, forward);
      basis.makeBasis(side, up, forward).scale(new THREE.Vector3(v.radius, v.radius, v.radius));
      basis.setPosition(v.position[0], v.position[1], v.position[2]);
      group.matrix.copy(basis);
      material.color.setHex(v.color);
    });
  }

  dispose(): void {
    this.body.dispose();
    this.mast.dispose();
    this.mastMaterial.dispose();
    for (const { material } of this.pool) material.dispose();
  }
}

/** Annotation lines and trails as one LineSegments with per-vertex colors. */
export class Lines3 {
  private readonly geometry = new THREE.BufferGeometry();
  private readonly material = new THREE.LineBasicMaterial({ vertexColors: true });
  private positions = new Float32Array(0);
  private colors = new Float32Array(0);
  private count = 0;

  constructor(scene: THREE.Scene) {
    const segments = new THREE.LineSegments(this.geometry, this.material);
    // The points change every frame, so a bounding sphere computed once goes stale.
    segments.frustumCulled = false;
    scene.add(segments);
  }

  set(lines: Line3View[], trails: Trail3View[]): void {
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

  private push(from: [number, number, number], to: [number, number, number], hex: number): void {
    color.setHex(hex);
    const i = this.count * 6;
    this.positions.set([from[0], from[1], from[2], to[0], to[1], to[2]], i);
    this.colors.set([color.r, color.g, color.b, color.r, color.g, color.b], i);
    this.count++;
  }
}

/** The demo's ground: a lit mesh over a TerrainView's height grid, rebuilt only when the view changes. */
export class Terrain3 {
  private mesh: THREE.Mesh<THREE.BufferGeometry, THREE.MeshLambertMaterial> | null = null;
  private shown: TerrainView | null = null;

  constructor(private readonly scene: THREE.Scene) {}

  /** Shows `view` (null: no ground); returns whether a ground is shown. */
  set(view: TerrainView | null): boolean {
    if (view === this.shown) return view !== null;
    this.shown = view;
    this.dispose();
    if (!view) return false;
    const { origin, spacing, size, heights } = view;
    const positions = new Float32Array(size * size * 3);
    for (let row = 0; row < size; row++) {
      for (let column = 0; column < size; column++) {
        const i = row * size + column;
        positions.set([origin[0] + column * spacing, heights[i], origin[1] + row * spacing], i * 3);
      }
    }
    // Two triangles per cell, wound so their normals point up (+y).
    const indices: number[] = [];
    for (let row = 0; row < size - 1; row++) {
      for (let column = 0; column < size - 1; column++) {
        const a = row * size + column;
        const below = a + size;
        indices.push(a, below, a + 1, a + 1, below, below + 1);
      }
    }
    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute("position", new THREE.BufferAttribute(positions, 3));
    geometry.setIndex(indices);
    geometry.computeVertexNormals();
    // Pushed back in depth, so trails and lines lying on the ground stay visible.
    const material = new THREE.MeshLambertMaterial({
      color: view.color,
      flatShading: true,
      polygonOffset: true,
      polygonOffsetFactor: 1,
      polygonOffsetUnits: 1,
    });
    this.mesh = new THREE.Mesh(geometry, material);
    this.scene.add(this.mesh);
    return true;
  }

  dispose(): void {
    if (!this.mesh) return;
    this.scene.remove(this.mesh);
    this.mesh.geometry.dispose();
    this.mesh.material.dispose();
    this.mesh = null;
  }
}

/** Text labels as HTML elements over the canvas (CSS2DRenderer). */
export class Labels3 {
  private readonly pool: CSS2DObject[] = [];

  constructor(private readonly scene: THREE.Scene) {}

  set(labels: Label3View[]): void {
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
      label.position.set(l.at[0], l.at[1], l.at[2]);
      label.element.textContent = l.text;
      label.element.style.color = `#${l.color.toString(16).padStart(6, "0")}`;
    });
  }
}
