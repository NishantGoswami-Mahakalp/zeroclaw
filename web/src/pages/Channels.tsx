import { useState, useEffect, useCallback, useMemo } from 'react';
import { Plus, Trash2, Edit2, Loader2, MessageSquare, Check } from 'lucide-react';
import {
  getChannels,
  createChannel,
  updateChannel,
  deleteChannel,
  getAllChannelSchemas,
  type Channel,
} from '@/lib/api';
import type { ChannelSchema, SchemaField } from '@/types/api';
import { SchemaFormWrapper } from '@/components/schema/SchemaForm';

interface ChannelTypeOption {
  value: string;
  label: string;
}

function getChannelTypes(): ChannelTypeOption[] {
  return [
    { value: 'cli', label: 'CLI' },
    { value: 'telegram', label: 'Telegram' },
    { value: 'discord', label: 'Discord' },
    { value: 'slack', label: 'Slack' },
    { value: 'mattermost', label: 'Mattermost' },
    { value: 'webhook', label: 'Webhook' },
    { value: 'imessage', label: 'iMessage' },
    { value: 'matrix', label: 'Matrix' },
    { value: 'signal', label: 'Signal' },
    { value: 'whatsapp', label: 'WhatsApp' },
    { value: 'linq', label: 'LINQ' },
    { value: 'nextcloud_talk', label: 'Nextcloud Talk' },
    { value: 'irc', label: 'IRC' },
    { value: 'lark', label: 'Lark' },
    { value: 'feishu', label: 'Feishu' },
    { value: 'dingtalk', label: 'DingTalk' },
    { value: 'qq', label: 'QQ' },
    { value: 'nostr', label: 'Nostr' },
    { value: 'clawdtalk', label: 'ClawdTalk' },
    { value: 'email', label: 'Email' },
  ];
}

export default function Channels() {
  const [channels, setChannels] = useState<Channel[]>([]);
  const [channelTypes, setChannelTypes] = useState<ChannelTypeOption[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [channelType, setChannelType] = useState('');
  const [isEnabled, setIsEnabled] = useState(true);
  const [formValues, setFormValues] = useState<Record<string, unknown>>({});

  const [schemaLoading, setSchemaLoading] = useState(false);
  const [currentSchema, setCurrentSchema] = useState<ChannelSchema | null>(null);
  const [schemaError, setSchemaError] = useState<string | null>(null);

  useEffect(() => {
    loadChannels();
    loadChannelTypes();
  }, []);

  const channelTypesList = useMemo(() => {
    if (channelTypes.length > 0) {
      return channelTypes;
    }
    return getChannelTypes();
  }, [channelTypes]);

  async function loadChannelTypes() {
    try {
      const data = await getAllChannelSchemas();
      if (data.channels && data.channels.length > 0) {
        const types = data.channels.map((c: ChannelSchema) => ({
          value: c.type,
          label: c.name || c.type,
        }));
        setChannelTypes(types);
      }
    } catch (e) {
      console.error('Failed to load channel types:', e);
    }
  }

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

  const fetchSchema = useCallback(async (type: string) => {
    if (!type) {
      setCurrentSchema(null);
      return;
    }

    setSchemaLoading(true);
    setSchemaError(null);

    try {
      const schema = await getAllChannelSchemas();
      const found = schema.channels?.find((c: ChannelSchema) => c.type === type);
      if (found) {
        setCurrentSchema(found);
        setFormValues({});
      } else {
        setSchemaError(`No schema found for channel type: ${type}`);
        setCurrentSchema(null);
      }
    } catch (e) {
      setSchemaError(e instanceof Error ? e.message : 'Failed to load schema');
      setCurrentSchema(null);
    } finally {
      setSchemaLoading(false);
    }
  }, []);

  function handleChannelTypeChange(type: string) {
    setChannelType(type);
    setFormValues({});
    if (type) {
      fetchSchema(type);
    } else {
      setCurrentSchema(null);
    }
  }

  function resetForm() {
    setChannelType('');
    setCurrentSchema(null);
    setFormValues({});
    setIsEnabled(true);
    setShowForm(false);
    setEditingId(null);
    setSchemaError(null);
  }

  async function handleSubmit(values: Record<string, unknown>) {
    if (!channelType.trim()) return;
    setSaving(true);
    setError(null);

    try {
      const configJson = JSON.stringify(values, null, 2);
      const channelData = {
        profile_id: 'default',
        channel_type: channelType.trim(),
        config: configJson,
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
    setIsEnabled(channel.is_enabled);
    setEditingId(channel.id);
    setShowForm(true);

    try {
      const parsed = JSON.parse(channel.config || '{}');
      setFormValues(parsed);
    } catch {
      setFormValues({});
    }

    fetchSchema(channel.channel_type);
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
                  onChange={(e) => handleChannelTypeChange(e.target.value)}
                  className="w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                  disabled={!!editingId}
                >
                  <option value="">Select channel...</option>
                  {channelTypesList.map((opt) => (
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

            {schemaLoading && (
              <div className="flex items-center gap-2 text-gray-400 py-4">
                <Loader2 className="h-4 w-4 animate-spin" />
                Loading schema...
              </div>
            )}

            {schemaError && (
              <div className="bg-red-900/20 border border-red-800 rounded-lg p-3 text-red-400 text-sm">
                {schemaError}
              </div>
            )}

            {currentSchema && !schemaLoading && (
              <div className="space-y-4">
                {currentSchema.description && (
                  <div className="bg-blue-900/20 border border-blue-800 rounded-lg p-3 text-blue-300 text-sm">
                    {currentSchema.description}
                  </div>
                )}

                <div className="text-sm text-gray-400">
                  <span className="font-medium text-gray-300">Required fields: </span>
                  {currentSchema.fields
                    .filter((f: SchemaField) => f.required)
                    .map((f: SchemaField) => f.name.replace(/_/g, ' '))
                    .join(', ') || 'None'}
                </div>

                <SchemaFormWrapper
                  schema={currentSchema}
                  initialValues={formValues}
                  onSubmit={handleSubmit}
                  onCancel={resetForm}
                  loading={saving}
                  disabled={saving}
                  submitLabel={editingId ? 'Update Channel' : 'Add Channel'}
                  cancelLabel="Cancel"
                />
              </div>
            )}

            {!currentSchema && !schemaLoading && channelType && (
              <div className="text-gray-500 text-sm py-4">
                No schema available for this channel type.
              </div>
            )}
          </div>
        </div>
      )}

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
                        {channelTypesList.find((c) => c.value === channel.channel_type)?.label ||
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
