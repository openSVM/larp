import { useSettings } from '../../lib/store/settings';

export function SettingsPanel() {
  const { settings, updateEditorPreferences, setActiveModel } = useSettings();

  return (
    <div className="settings-panel p-4 bg-gray-800 text-white">
      <h2 className="text-xl font-bold mb-4">Settings</h2>
      
      <section className="mb-6">
        <h3 className="text-lg font-semibold mb-2">Model Selection</h3>
        <select 
          value={settings.activeModel}
          onChange={(e) => setActiveModel(e.target.value)}
          className="w-full p-2 bg-gray-700 rounded border border-gray-600"
        >
          {settings.models.map(model => (
            <option key={model.name} value={model.name}>
              {model.name} ({model.provider})
            </option>
          ))}
        </select>
      </section>

      <section className="mb-6">
        <h3 className="text-lg font-semibold mb-2">Editor Preferences</h3>
        <div className="space-y-4">
          <div>
            <label className="block mb-1">Theme</label>
            <select
              value={settings.editorPreferences.theme}
              onChange={(e) => updateEditorPreferences({ theme: e.target.value })}
              className="w-full p-2 bg-gray-700 rounded border border-gray-600"
            >
              <option value="dark">Dark</option>
              <option value="light">Light</option>
            </select>
          </div>
          
          <div>
            <label className="block mb-1">Font Size</label>
            <input
              type="number"
              value={settings.editorPreferences.fontSize}
              onChange={(e) => updateEditorPreferences({ fontSize: parseInt(e.target.value) })}
              className="w-full p-2 bg-gray-700 rounded border border-gray-600"
              min="8"
              max="32"
            />
          </div>

          <div>
            <label className="block mb-1">Tab Size</label>
            <input
              type="number"
              value={settings.editorPreferences.tabSize}
              onChange={(e) => updateEditorPreferences({ tabSize: parseInt(e.target.value) })}
              className="w-full p-2 bg-gray-700 rounded border border-gray-600"
              min="2"
              max="8"
            />
          </div>
        </div>
      </section>
    </div>
  );
}