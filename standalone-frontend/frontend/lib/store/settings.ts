import create from 'zustand';

interface EditorPreferences {
  theme: string;
  fontSize: number;
  tabSize: number;
}

interface Model {
  name: string;
  provider: string;
  contextLength: number;
}

interface Settings {
  activeModel: string;
  models: Model[];
  editorPreferences: EditorPreferences;
}

interface SettingsStore {
  settings: Settings;
  updateSettings: (settings: Partial<Settings>) => void;
  updateEditorPreferences: (prefs: Partial<EditorPreferences>) => void;
  setActiveModel: (modelName: string) => void;
}

export const useSettings = create<SettingsStore>((set) => ({
  settings: {
    activeModel: 'gpt-4',
    models: [
      {
        name: 'gpt-4',
        provider: 'openai',
        contextLength: 8192
      },
      {
        name: 'claude-2',
        provider: 'anthropic',
        contextLength: 100000
      }
    ],
    editorPreferences: {
      theme: 'dark',
      fontSize: 14,
      tabSize: 2
    }
  },
  updateSettings: (newSettings) =>
    set((state) => ({
      settings: { ...state.settings, ...newSettings }
    })),
  updateEditorPreferences: (prefs) =>
    set((state) => ({
      settings: {
        ...state.settings,
        editorPreferences: { ...state.settings.editorPreferences, ...prefs }
      }
    })),
  setActiveModel: (modelName) =>
    set((state) => ({
      settings: { ...state.settings, activeModel: modelName }
    }))
}));