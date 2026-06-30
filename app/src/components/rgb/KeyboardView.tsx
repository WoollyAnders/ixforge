import { useId } from "react";
import type { LedLayout } from "../../types/forge";
import type { ChassisSpec } from "../../rgb/deviceArt";

const UNIT = 42; // px per key unit
const GAP = 4; // px gap between keycaps
const OFF = "#1b1922"; // unlit keycap

export interface KeyboardViewProps {
  layout: LedLayout;
  chassis: ChassisSpec;
  /** keyId -> hex; absent keys render unlit. */
  colors: Record<string, string>;
  /** When provided, keys are clickable (editor mode). */
  onKeyClick?: (keyId: string) => void;
  /** Tint for the knob ring, screen text, and side-light glow. */
  accent?: string;
  /** Small text shown on the LCD screen. */
  screenText?: string;
  /** LCD width:height ratio (from the device's display) — sizes the screen rect. */
  screenAspect?: number;
  /** Global brightness multiplier (0..1) applied to displayed key colors. */
  brightness?: number;
}

// Procedurally drawn keyboard rendition — no image assets. Keys/case/knob/screen
// are SVG shapes positioned from the layout coordinates + the chassis spec, so
// the same component renders any device from its data.
export function KeyboardView({
  layout,
  chassis,
  colors,
  onKeyClick,
  accent = "#22d3ee",
  screenText = "IX FORGE",
  screenAspect = 16 / 9,
  brightness = 1,
}: KeyboardViewProps) {
  const uid = useId().replace(/:/g, "");
  const U = (v: number) => v * UNIT;

  const maxX = Math.max(...layout.keys.map((k) => k.x + k.w), 1);
  const maxY = Math.max(...layout.keys.map((k) => k.y + k.h), 1);
  const { padding: p } = chassis;
  const caseX0 = -p.left;
  const caseY0 = -p.top;
  const caseW = maxX + p.left + p.right;
  const caseH = maxY + p.top + p.bottom;

  const interactive = typeof onKeyClick === "function";
  const litKeys = layout.keys.filter((k) => colors[k.id]);
  // Screen width derived from the panel's aspect ratio (data-driven from the LCD).
  const screenW = chassis.screen ? chassis.screen.h * screenAspect : 0;

  return (
    <div style={{ overflowX: "auto", width: "100%" }}>
      <svg
        viewBox={`${U(caseX0)} ${U(caseY0)} ${U(caseW)} ${U(caseH)}`}
        width={U(caseW)}
        height={U(caseH)}
        style={{ maxWidth: "100%", height: "auto", display: "block" }}
        role="img"
        aria-label="Keyboard lighting preview"
      >
        <defs>
          <linearGradient id={`case-${uid}`} x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor={chassis.caseTop} />
            <stop offset="100%" stopColor={chassis.caseBottom} />
          </linearGradient>
          <radialGradient id={`knob-${uid}`} cx="38%" cy="32%" r="75%">
            <stop offset="0%" stopColor="#6c6a76" />
            <stop offset="55%" stopColor="#34323c" />
            <stop offset="100%" stopColor="#17161c" />
          </radialGradient>
          <filter id={`glow-${uid}`} x="-60%" y="-60%" width="220%" height="220%">
            <feGaussianBlur stdDeviation="3" />
          </filter>
          <filter id={`side-${uid}`} x="-80%" y="-80%" width="260%" height="260%">
            <feGaussianBlur stdDeviation="9" />
          </filter>
        </defs>

        {/* Side-light glow under the case edges */}
        {chassis.sidelight && (
          <g filter={`url(#side-${uid})`} opacity={0.5}>
            <rect x={U(caseX0)} y={U(caseY0 + 0.3)} width={U(0.25)} height={U(caseH - 0.6)} fill={accent} />
            <rect x={U(caseX0 + caseW - 0.25)} y={U(caseY0 + 0.3)} width={U(0.25)} height={U(caseH - 0.6)} fill={accent} />
            <rect x={U(caseX0 + 0.4)} y={U(caseY0 + caseH - 0.25)} width={U(caseW - 0.8)} height={U(0.25)} fill={accent} />
          </g>
        )}

        {/* Case */}
        <rect
          x={U(caseX0)}
          y={U(caseY0)}
          width={U(caseW)}
          height={U(caseH)}
          rx={chassis.cornerRadius}
          fill={`url(#case-${uid})`}
          stroke="#0e0d12"
          strokeWidth={1.5}
        />

        {/* Status indicator LEDs (top-left) */}
        {chassis.indicators &&
          Array.from({ length: chassis.indicators.count }).map((_, i) => (
            <circle
              key={`ind-${i}`}
              cx={U(chassis.indicators!.x + i * 0.32)}
              cy={U(chassis.indicators!.y)}
              r={3}
              fill={i === 0 ? accent : "#54515f"}
              opacity={i === 0 ? 0.9 : 0.6}
            />
          ))}

        {/* Glow halo behind lit keys */}
        <g filter={`url(#glow-${uid})`} opacity={0.85}>
          {litKeys.map((k) => (
            <rect
              key={`glow-${k.id}`}
              x={U(k.x) + GAP / 2}
              y={U(k.y) + GAP / 2}
              width={U(k.w) - GAP}
              height={U(k.h) - GAP}
              rx={6}
              fill={dimHex(colors[k.id], brightness)}
            />
          ))}
        </g>

        {/* Keycaps */}
        {layout.keys.map((k) => {
          const fill = colors[k.id] ? dimHex(colors[k.id], brightness) : OFF;
          const x = U(k.x) + GAP / 2;
          const y = U(k.y) + GAP / 2;
          const w = U(k.w) - GAP;
          const h = U(k.h) - GAP;
          return (
            <g
              key={k.id}
              onClick={interactive ? () => onKeyClick!(k.id) : undefined}
              style={interactive ? { cursor: "pointer" } : undefined}
            >
              <rect
                className={interactive ? "kbd-key" : undefined}
                x={x}
                y={y}
                width={w}
                height={h}
                rx={5}
                fill={fill}
                stroke="#0f0e14"
                strokeWidth={1}
              />
              {k.label && (
                <text
                  x={x + w / 2}
                  y={y + h / 2}
                  fill={isDark(fill) ? "#cfcbe0" : "#101013"}
                  fontSize={9}
                  textAnchor="middle"
                  dominantBaseline="central"
                  style={{ pointerEvents: "none", userSelect: "none" }}
                >
                  {k.label}
                </text>
              )}
            </g>
          );
        })}

        {/* LCD screen (width matches the panel aspect ratio) */}
        {chassis.screen && (
          <g>
            <clipPath id={`scr-${uid}`}>
              <rect
                x={U(chassis.screen.x)}
                y={U(chassis.screen.y)}
                width={U(screenW)}
                height={U(chassis.screen.h)}
                rx={4}
              />
            </clipPath>
            <rect
              x={U(chassis.screen.x)}
              y={U(chassis.screen.y)}
              width={U(screenW)}
              height={U(chassis.screen.h)}
              rx={4}
              fill="#07070d"
              stroke="#3a3846"
              strokeWidth={1}
            />
            <text
              x={U(chassis.screen.x + screenW / 2)}
              y={U(chassis.screen.y + chassis.screen.h / 2)}
              clipPath={`url(#scr-${uid})`}
              fill={accent}
              fontSize={9}
              fontFamily="monospace"
              textAnchor="middle"
              dominantBaseline="central"
              style={{ pointerEvents: "none", userSelect: "none" }}
            >
              {screenText}
            </text>
          </g>
        )}

        {/* Volume knob */}
        {chassis.knob && (
          <g>
            <circle
              cx={U(chassis.knob.x)}
              cy={U(chassis.knob.y)}
              r={U(chassis.knob.r)}
              fill={`url(#knob-${uid})`}
              stroke={accent}
              strokeWidth={1.5}
              strokeOpacity={0.5}
            />
            <line
              x1={U(chassis.knob.x)}
              y1={U(chassis.knob.y)}
              x2={U(chassis.knob.x)}
              y2={U(chassis.knob.y - chassis.knob.r * 0.7)}
              stroke="#d7d4e2"
              strokeWidth={2}
              strokeLinecap="round"
            />
          </g>
        )}
      </svg>
    </div>
  );
}

function isDark(hex: string): boolean {
  const h = hex.replace("#", "");
  if (h.length < 6) return true;
  const r = parseInt(h.slice(0, 2), 16);
  const g = parseInt(h.slice(2, 4), 16);
  const b = parseInt(h.slice(4, 6), 16);
  return (r * 299 + g * 587 + b * 114) / 1000 < 140;
}

function dimHex(hex: string, f: number): string {
  if (f >= 1) return hex;
  const h = hex.replace("#", "");
  const ch = (i: number) =>
    Math.round(parseInt(h.slice(i, i + 2), 16) * Math.max(0, f))
      .toString(16)
      .padStart(2, "0");
  return `#${ch(0)}${ch(2)}${ch(4)}`;
}
