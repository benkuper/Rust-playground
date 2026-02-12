import OutlinerPanel from "./core/OutlinerPanel.svelte";
import InspectorPanel from "./core/InspectorPanel.svelte";
import ParamsPanel from "./core/ParamsPanel.svelte";
import EventsPanel from "./core/EventsPanel.svelte";
import StatsPanel from "./core/StatsPanel.svelte";
import { appPanels, type PanelDefinition } from "../app/panels";

export const corePanels: PanelDefinition[] = [
  {
    id: "outliner",
    title: "Outliner",
    component: OutlinerPanel,
    defaultOpen: true
  },
  {
    id: "inspector",
    title: "Inspector",
    component: InspectorPanel,
    defaultOpen: true
  },
  {
    id: "params",
    title: "Parameters",
    component: ParamsPanel,
    defaultOpen: true
  },
  {
    id: "events",
    title: "Events",
    component: EventsPanel,
    defaultOpen: false
  },
  {
    id: "stats",
    title: "Engine Pulse",
    component: StatsPanel,
    defaultOpen: true
  }
];

export const panels = [...corePanels, ...appPanels];
