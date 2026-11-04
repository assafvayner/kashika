import { Float, type Json } from "../diag/json.js";
import { hypot } from "../diag/rust.js";

export const TOLERANCE = 0.5;

export class Pt {
  constructor(
    readonly x: number,
    readonly y: number,
  ) {}

  static new(x: number, y: number): Pt {
    return new Pt(x, y);
  }

  static default(): Pt {
    return new Pt(0, 0);
  }

  length(): number {
    return hypot(this.x, this.y);
  }

  dist(other: Pt): number {
    return this.sub(other).length();
  }

  norm(): Pt {
    const l = this.length();
    if (l === 0) {
      return this;
    } else {
      return new Pt(this.x / l, this.y / l);
    }
  }

  lerp(other: Pt, t: number): Pt {
    return this.add(other.sub(this).mul(t));
  }

  add(o: Pt): Pt {
    return new Pt(this.x + o.x, this.y + o.y);
  }

  sub(o: Pt): Pt {
    return new Pt(this.x - o.x, this.y - o.y);
  }

  mul(k: number): Pt {
    return new Pt(this.x * k, this.y * k);
  }

  eq(o: Pt): boolean {
    return this.x === o.x && this.y === o.y;
  }

  toJson(): Json {
    return { x: new Float(this.x), y: new Float(this.y) };
  }
}
