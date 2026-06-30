import { useStore } from "../../store/useStore";
import type { LedLayout } from "../../types/forge";

const UNIT = 44; // px per key unit
const GAP = 4;
const OFF_COLOR = "#1b1d23";

// Renders any layout from data — a 60% board and a full-size board both "just
// work" because positions come from the profile's LedLayout.
export function KeyboardCanvas({ layout }: { layout: LedLayout }) {
  const keyColors = useStore((s) => s.keyColors);
  const paintKey = useStore((s) => s.paintKey);

  const width = Math.max(...layout.keys.map((k) => k.x + k.w), 1) * UNIT;
  const height = Math.max(...layout.keys.map((k) => k.y + k.h), 1) * UNIT;

  return (
    <div style={{ overflowX: "auto" }}>
      <svg
        width={width}
        height={height}
        viewBox={`0 0 ${width} ${height}`}
        style={{ maxWidth: "100%", height: "auto" }}
      >
        {layout.keys.map((k) => {
          const fill = keyColors[k.id] ?? OFF_COLOR;
          const x = k.x * UNIT + GAP / 2;
          const y = k.y * UNIT + GAP / 2;
          const w = k.w * UNIT - GAP;
          const h = k.h * UNIT - GAP;
          return (
            <g key={k.id} onClick={() => paintKey(k.id)}>
              <rect
                className="kbd-key"
                x={x}
                y={y}
                width={w}
                height={h}
                rx={5}
                fill={fill}
                stroke="#33363f"
              />
              <text
                x={x + w / 2}
                y={y + h / 2}
                fill={isDark(fill) ? "#c7c9d1" : "#101013"}
                fontSize={10}
                textAnchor="middle"
                dominantBaseline="central"
                style={{ pointerEvents: "none", userSelect: "none" }}
              >
                {k.label}
              </text>
            </g>
          );
        })}
      </svg>
    </div>
  );
}

function isDark(hex: string): boolean {
  const h = hex.replace("#", "");
  const r = parseInt(h.slice(0, 2), 16);
  const g = parseInt(h.slice(2, 4), 16);
  const b = parseInt(h.slice(4, 6), 16);
  return (r * 299 + g * 587 + b * 114) / 1000 < 130;
}
