import type { ComponentType } from "svelte";

export type PanelDefinition = {
  id: string;
  title: string;
  component: ComponentType;
  defaultOpen?: boolean;
};

export const appPanels: PanelDefinition[] = [];
