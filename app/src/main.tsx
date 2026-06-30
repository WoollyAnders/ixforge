import React from "react";
import ReactDOM from "react-dom/client";
import {
  MantineProvider,
  createTheme,
  type MantineColorsTuple,
} from "@mantine/core";
import "@mantine/core/styles.css";
import "./styles.css";
import App from "./App";

// Cyan-blue accent.
const brand: MantineColorsTuple = [
  "#e3fbff",
  "#c7f3ff",
  "#9fe9fc",
  "#6fdcf8",
  "#46d2f3",
  "#22cce8", // primary (dark scheme)
  "#10b6d6", // primary (light scheme)
  "#008fb0",
  "#00788f",
  "#005f72",
];

// Purple-tinted dark neutrals: surfaces, borders, and body background.
const dark: MantineColorsTuple = [
  "#e8e5f5", // text
  "#cbc6e0",
  "#a79fc4", // dimmed text
  "#7d75a3",
  "#544c74", // borders
  "#3a3357",
  "#241f3a", // elevated surfaces
  "#1c1830", // body background
  "#161227",
  "#100d20",
];

const theme = createTheme({
  primaryColor: "brand",
  primaryShade: { light: 6, dark: 5 },
  fontFamily: "Inter, system-ui, Avenir, Helvetica, Arial, sans-serif",
  colors: { brand, dark },
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <MantineProvider theme={theme} defaultColorScheme="dark">
      <App />
    </MantineProvider>
  </React.StrictMode>,
);
