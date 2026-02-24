import { useState, useEffect } from 'react';
import { Plus, Trash2, Edit2, Loader2, MessageSquare, Check } from 'lucide-react';
import {
  getChannels,
  createChannel,
  updateChannel,
  deleteChannel,
  type Channel,
} from '@/lib/api';

const CHANNEL_OPTIONS = [
  { value: 'telegram', label: 'Telegram' },
  { value: 'discord', label: 'Discord' },
  { value: 'slack', label: 'Slack' },
  { value: 'whatsapp', label: 'WhatsApp' },
  { value: 'signal', label: 'Signal' },
  { value: 'email', label: 'Email' },
  { value: 'webhook', label: 'Webhook' },
  { value: 'matrix', label: 'Matrix' },
  { value: 'irc', label: 'IRC' },
  { value: 'mattermost', label: 'Mattermost' },
  { value: 'feishu', label: 'Feishu/Lark' },
  { value: 'dingtalk', label: 'DingTalk' },
  { value: 'qq', label: 'QQ' },
  { value: 'nostr', label: 'Nostr' },
  { value: 'imessage', label: 'iMessage' },
  { value: 'nextcloud_talk', label: 'Nextcloud Talk' },
  { value: 'linq', label: 'LINQ' },
  { value: 'clawdtalk', label: 'ClawdTalk' },
];

export default function Channels() {
  const [channels, setChannels] = useState<Channel[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [channelType, setChannelType] = useState('');
  const [config, setConfig] = useState('');
  const [isEnabled, setIsEnabled] = useState(true);

  useEffect(() => {
    loadChannels();
  }, []);

  async function loadChannels() {
    try {
      const data = await getChannels();
      setChannels(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load channels');
    } finally {
      setLoading(false);
    }
  }

  function resetForm() {
    setChannelType('');
    setConfig('');
    setIsEnabled(true);
    setShowForm(false);
    setEditingId(null);
  }

  async function handleSubmit() {
    if (!channelType.trim()) return;
    setSaving(true);
    setError(null);

    try {
      const channelData = {
        profile_id: 'default',
        channel_type: channelType.trim(),
        config: config.trim() || '{}',
        is_enabled: isEnabled,
      };

      if (editingId) {
        await updateChannel(editingId, channelData);
      } else {
        await createChannel(channelData);
      }

      resetForm();
      loadChannels();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to save channel');
    } finally {
      setSaving(false);
    }
  }

  function startEdit(channel: Channel) {
    setChannelType(channel.channel_type);
    setConfig(channel.config);
    setIsEnabled(channel.is_enabled);
    setEditingId(channel.id);
    setShowForm(true);
  }

  async function handleDelete(id: string) {
    if (!confirm('Are you sure you want to delete this channel?')) return;
    setSaving(true);
    setError(null);
    try {
      await deleteChannel(id);
      loadChannels();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete channel');
    } finally {
      setSaving(false);
    }
  }

  async function handleToggle(id: string, currentEnabled: boolean) {
    setSaving(true);
    setError(null);
    try {
      await updateChannel(id, { is_enabled: !currentEnabled });
      loadChannels();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to toggle channel');
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
          <MessageSquare className="h-6 w-6 text-blue-400" />
          <h1 className="text-2xl font-bold text-white">Channels</h1>
        </div>
        <button
          onClick={() => setShowForm(true)}
          className="flex items-center gap-2 bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg transition-colors"
        >
          <Plus className="h-4 w-4" />
          Add Channel
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
            {editingId ? 'Edit Channel' : 'Add New Channel'}
          </h2>
          <div className="grid gap-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-1">
                  Channel Type
                </label>
                <select
                  value={channelType}
                  onChange={(e) => setChannelType(e.target.value)}
                  className="w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                  disabled={!!editingId}
                >
                  <option value="">Select channel...</option>
                  {CHANNEL_OPTIONS.map((opt) => (
                    <option key={opt.value} value={opt.value}>
                      {opt.label}
                    </option>
                  ))}
                </select>
              </div>
              <div className="flex items-end">
                <label className="flex items-center gap-2 text-gray-300 pb-2">
                  <input
                    type="checkbox"
                    checked={isEnabled}
                    onChange={(e) => setIsEnabled(e.target.checked)}
                    className="rounded bg-gray-800 border-gray-700 text-blue-600 focus:ring-blue-500"
                  />
                  Enabled
                </label>
              </div>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-1">
                Configuration (JSON)
              </label>
              <textarea
                value={config}
                onChange={(e) => setConfig(e.target.value)}
                placeholder='{"bot_token": "xxx", "allowed_users": ["*"]}'
                rows={4}
                className="w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-white placeholder-gray-500 font-mono text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
            <div className="flex gap-3">
              <button
                onClick={handleSubmit}
                disabled={saving || !channelType.trim()}
                className="flex items-center gap-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white px-4 py-2 rounded-lg transition-colors"
              >
                {saving ? <Loader2 className="h-4 w-4 animate-spin" /> : <Check className="h-4 w-4" />}
                {editingId ? 'Update' : 'Add'} Channel
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

      {/* Channel list */}
      <div className="bg-gray-900 rounded-xl border border-gray-800 p-4">
        <h2 className="text-lg font-semibold text-white mb-4">Configured Channels</h2>
        {channels.length === 0 ? (
          <p className="text-gray-500">No channels configured. Add one to get started.</p>
        ) : (
          <div className="grid gap-3 md:grid-cols-2 lg:grid-cols-3">
            {channels.map((channel) => (
              <div
                key={channel.id}
                className={`p-4 rounded-lg border ${
                  channel.is_enabled
                    ? 'bg-gray-800 border-gray-700'
                    : 'bg-gray-900/50 border-gray-800 opacity-60'
                }`}
              >
                <div className="flex items-start justify-between">
                  <div className="flex-1">
                    <div className="flex items-center gap-2">
                      <h3 className="font-semibold text-white">
                        {CHANNEL_OPTIONS.find((c) => c.value === channel.channel_type)?.label ||
                          channel.channel_type}
                      </h3>
                    </div>
                    <p className="text-xs text-gray-500 mt-1">
                      {channel.is_enabled ? 'Active' : 'Disabled'}
                    </p>
                  </div>
                  <div className="flex items-center gap-1">
                    <button
                      onClick={() => handleToggle(channel.id, channel.is_enabled)}
                      disabled={saving}
                      className={`p-1.5 rounded ${
                        channel.is_enabled
                          ? 'hover:bg-green-900/30 text-green-400'
                          : 'hover:bg-gray-700 text-gray-400'
                      }`}
                      title={channel.is_enabled ? 'Disable' : 'Enable'}
                    >
                      <Check className="h-4 w-4" />
                    </button>
                    <button
                      onClick={() => startEdit(channel)}
                      className="p-1.5 hover:bg-gray-700 text-gray-400 hover:text-white rounded"
                    >
                      <Edit2 className="h-4 w-4" />
                    </button>
                    <button
                      onClick={() => handleDelete(channel.id)}
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
