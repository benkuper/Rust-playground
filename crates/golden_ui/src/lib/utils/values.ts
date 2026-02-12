type Vec2 = { x: number; y: number };
type Vec3 = { x: number; y: number; z: number };
type ColorRgba = { r: number; g: number; b: number; a: number };

type ValueObject = Record<string, any>;
export type ValueInput = ValueObject | string | null | undefined;

export function unwrapValue(value: ValueInput) {
  if (value && typeof value === "object") {
    const key = Object.keys(value)[0];
    return { kind: key, value: (value as ValueObject)[key] };
  }
  if (typeof value === "string") {
    return { kind: value, value: null };
  }
  return { kind: "Unknown", value } as { kind: string; value: any };
}

export function formatValue(value: ValueInput) {
  const { kind, value: inner } = unwrapValue(value);
  if (kind === "Vec2") {
    const vec = inner as Vec2;
    return `x:${vec.x} y:${vec.y}`;
  }
  if (kind === "Vec3") {
    const vec = inner as Vec3;
    return `x:${vec.x} y:${vec.y} z:${vec.z}`;
  }
  if (kind === "ColorRgba") {
    const color = inner as ColorRgba;
    return `rgba(${color.r}, ${color.g}, ${color.b}, ${color.a})`;
  }
  if (kind === "Enum") return `${inner.enum_id}::${inner.variant}`;
  if (kind === "Reference") return inner.uuid;
  if (kind === "Trigger") return "Trigger";
  return String(inner ?? "");
}

export function buildValue(kind: string, value: unknown) {
  if (kind === "Trigger") {
    return "Trigger";
  }
  return { [kind]: value };
}

export function constraintBounds(constraints: unknown) {
  if (!constraints || typeof constraints !== "object") {
    return {} as Record<string, unknown>;
  }
  if (constraints === "None") {
    return {} as Record<string, unknown>;
  }
  const typed = constraints as ValueObject;
  if (typed.Int) return typed.Int;
  if (typed.Float) return typed.Float;
  if (typed.String) return typed.String;
  return {} as Record<string, unknown>;
}
