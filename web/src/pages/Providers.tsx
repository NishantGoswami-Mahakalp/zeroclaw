import { useState, useEffect } from 'react';
import { Plus, Trash2, Edit2, Loader2, Cloud, Check } from 'lucide-react';
import {
  getProviders,
  createProvider,
  updateProvider,
  deleteProvider,
  type Provider,
} from '@/lib/api';

const PROVIDER_OPTIONS = [
  { value: 'openai', label: 'OpenAI' },
  { value: 'anthropic', label: 'Anthropic' },
  { value: 'google', label: 'Google (Gemini)' },
  { value: 'openrouter', label: 'OpenRouter' },
  { value: 'ollama', label: 'Ollama' },
  { value: 'groq', label: 'Groq' },
  { value: 'deepseek', label: 'DeepSeek' },
  { value: 'mistral', label: 'Mistral' },
  { value: 'xai', label: 'xAI' },
  { value: 'together', label: 'Together AI' },
  { value: 'fireworks', label: 'Fireworks AI' },
  { value: 'perplexity', label: 'Perplexity' },
  { value: 'cohere', label: 'Cohere' },
  { value: 'qwen', label: 'Qwen' },
  { value: 'glm', label: 'GLM' },
  { value: 'moonshot', label: 'Moonshot' },
  { value: 'minimax', label: 'MiniMax' },
];

export default function Providers() {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [name, setName] = useState('');
  const [apiKey, setApiKey] = useState('');
  const [apiUrl, setApiUrl] = useState('');
  const [defaultModel, setDefaultModel] = useState('');
  const [isDefault, setIsDefault] = useState(false);
  const [isEnabled, setIsEnabled] = useState(true);

  useEffect(() => {
    loadProviders();
  }, []);

  async function loadProviders() {
    try {
      const data = await getProviders();
      setProviders(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load providers');
    } finally {
      setLoading(false);
    }
  }

  function resetForm() {
    setName('');
    setApiKey('');
    setApiUrl('');
    setDefaultModel('');
    setIsDefault(false);
    setIsEnabled(true);
    setShowForm(false);
    setEditingId(null);
  }

  async function handleSubmit() {
    if (!name.trim()) return;
    setSaving(true);
    setError(null);

    try {
      const providerData = {
        profile_id: 'default',
        name: name.trim(),
        api_key: apiKey.trim() || undefined,
        api_url: apiUrl.trim() || undefined,
        default_model: defaultModel.trim() || undefined,
        is_default: isDefault,
        is_enabled: isEnabled,
        priority: 0,
      };

      if (editingId) {
        await updateProvider(editingId, providerData);
      } else {
        await createProvider(providerData);
      }

      resetForm();
      loadProviders();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to save provider');
    } finally {
      setSaving(false);
    }
  }

  function startEdit(provider: Provider) {
    setName(provider.name);
    setApiKey(provider.api_key || '');
    setApiUrl(provider.api_url || '');
    setDefaultModel(provider.default_model || '');
    setIsDefault(provider.is_default);
    setIsEnabled(provider.is_enabled);
    setEditingId(provider.id);
    setShowForm(true);
  }

  async function handleDelete(id: string) {
    if (!confirm('Are you sure you want to delete this provider?')) return;
    setSaving(true);
    setError(null);
    try {
      await deleteProvider(id);
      loadProviders();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete provider');
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="h-8 w-8 animate-spin text-blue-500" />
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Cloud className="h-6 w-6 text-blue-400" />
          <h1 className="text-2xl font-bold text-white">AI Providers</h1>
        </div>
        <button
          onClick={() => setShowForm(true)}
          className="flex items-center gap-2 bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg transition-colors"
        >
          <Plus className="h-4 w-4" />
          Add Provider
        </button>
      </div>

      {error && (
        <div className="bg-red-900/30 border border-red-700 rounded-lg p-3 text-red-300">
          {error}
        </div>
      )}

      {/* Add/Edit form */}
      {showForm && (
        <div className="bg-gray-900 rounded-xl border border-gray-800 p-4">
          <h2 className="text-lg font-semibold text-white mb-4">
            {editingId ? 'Edit Provider' : 'Add New Provider'}
          </h2>
          <div className="grid gap-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-1">
                  Provider
                </label>
                <select
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className="w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                >
                  <option value="">Select provider...</option>
                  {PROVIDER_OPTIONS.map((opt) => (
                    <option key={opt.value} value={opt.value}>
                      {opt.label}
                    </option>
                  ))}
                </select>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-1">
                  Default Model
                </label>
                <input
                  type="text"
                  value={defaultModel}
                  onChange={(e) => setDefaultModel(e.target.value)}
                  placeholder="e.g., gpt-4o, claude-sonnet-4"
                  className="w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-1">
                API Key
              </label>
              <input
                type="password"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
                placeholder="Leave empty to use environment variable"
                className="w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-1">
                API URL (optional)
              </label>
              <input
                type="url"
                value={apiUrl}
                onChange={(e) => setApiUrl(e.target.value)}
                placeholder="e.g., http://localhost:11434 (for Ollama)"
                className="w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
            <div className="flex items-center gap-6">
              <label className="flex items-center gap-2 text-gray-300">
                <input
                  type="checkbox"
                  checked={isDefault}
                  onChange={(e) => setIsDefault(e.target.checked)}
                  className="rounded bg-gray-800 border-gray-700 text-blue-600 focus:ring-blue-500"
                />
                Default Provider
              </label>
              <label className="flex items-center gap-2 text-gray-300">
                <input
                  type="checkbox"
                  checked={isEnabled}
                  onChange={(e) => setIsEnabled(e.target.checked)}
                  className="rounded bg-gray-800 border-gray-700 text-blue-600 focus:ring-blue-500"
                />
                Enabled
              </label>
            </div>
            <div className="flex gap-3">
              <button
                onClick={handleSubmit}
                disabled={saving || !name.trim()}
                className="flex items-center gap-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white px-4 py-2 rounded-lg transition-colors"
              >
                {saving ? <Loader2 className="h-4 w-4 animate-spin" /> : <Check className="h-4 w-4" />}
                {editingId ? 'Update' : 'Add'} Provider
              </button>
              <button
                onClick={resetForm}
                className="text-gray-400 hover:text-white px-4 py-2 rounded-lg transition-colors"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Provider list */}
      <div className="bg-gray-900 rounded-xl border border-gray-800 p-4">
        <h2 className="text-lg font-semibold text-white mb-4">Configured Providers</h2>
        {providers.length === 0 ? (
          <p className="text-gray-500">No providers configured. Add one to get started.</p>
        ) : (
          <div className="grid gap-3 md:grid-cols-2 lg:grid-cols-3">
            {providers.map((provider) => (
              <div
                key={provider.id}
                className={`p-4 rounded-lg border ${
                  provider.is_enabled
                    ? 'bg-gray-800 border-gray-700'
                    : 'bg-gray-900/50 border-gray-800 opacity-60'
                }`}
              >
                <div className="flex items-start justify-between">
                  <div>
                    <div className="flex items-center gap-2">
                      <h3 className="font-semibold text-white">
                        {PROVIDER_OPTIONS.find((p) => p.value === provider.name)?.label || provider.name}
                      </h3>
                      {provider.is_default && (
                        <span className="px-2 py-0.5 text-xs bg-blue-700 text-blue-200 rounded">
                          Default
                        </span>
                      )}
                    </div>
                    {provider.default_model && (
                      <p className="text-sm text-gray-400 mt-1">{provider.default_model}</p>
                    )}
                  </div>
                  <div className="flex items-center gap-1">
                    <button
                      onClick={() => startEdit(provider)}
                      className="p-1.5 hover:bg-gray-700 text-gray-400 hover:text-white rounded"
                    >
                      <Edit2 className="h-4 w-4" />
                    </button>
                    <button
                      onClick={() => handleDelete(provider.id)}
                      className="p-1.5 hover:bg-red-900/30 text-gray-400 hover:text-red-400 rounded"
                    >
                      <Trash2 className="h-4 w-4" />
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
