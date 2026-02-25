import { useState, useEffect } from 'react';
import { Plus, Trash2, Check, Loader2, Server } from 'lucide-react';
import { getProfiles, createProfile, activateProfile, deleteProfile, type Profile } from '@/lib/api';

export default function Profiles() {
  const [profiles, setProfiles] = useState<Profile[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [newName, setNewName] = useState('');
  const [newDescription, setNewDescription] = useState('');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadProfiles();
  }, []);

  async function loadProfiles() {
    try {
      const data = await getProfiles();
      setProfiles(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load profiles');
    } finally {
      setLoading(false);
    }
  }

  async function handleCreate() {
    if (!newName.trim()) return;
    setSaving(true);
    setError(null);
    try {
      await createProfile(newName.trim(), newDescription.trim() || undefined);
      setNewName('');
      setNewDescription('');
      loadProfiles();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to create profile');
    } finally {
      setSaving(false);
    }
  }

  async function handleActivate(id: string) {
    setSaving(true);
    setError(null);
    try {
      await activateProfile(id);
      loadProfiles();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to activate profile');
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete(id: string) {
    if (!confirm('Are you sure you want to delete this profile?')) return;
    setSaving(true);
    setError(null);
    try {
      await deleteProfile(id);
      loadProfiles();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete profile');
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
      <div className="flex items-center gap-3">
        <Server className="h-6 w-6 text-blue-400" />
        <h1 className="text-2xl font-bold text-white">Profiles</h1>
      </div>

      {error && (
        <div className="bg-red-900/30 border border-red-700 rounded-lg p-3 text-red-300">
          {error}
        </div>
      )}

      {/* Create new profile */}
      <div className="bg-gray-900 rounded-xl border border-gray-800 p-4">
        <h2 className="text-lg font-semibold text-white mb-4">Create Profile</h2>
        <div className="flex gap-3">
          <input
            type="text"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            placeholder="Profile name"
            className="flex-1 bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <input
            type="text"
            value={newDescription}
            onChange={(e) => setNewDescription(e.target.value)}
            placeholder="Description (optional)"
            className="flex-1 bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-white placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
          <button
            onClick={handleCreate}
            disabled={saving || !newName.trim()}
            className="flex items-center gap-2 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 text-white px-4 py-2 rounded-lg transition-colors"
          >
            {saving ? <Loader2 className="h-4 w-4 animate-spin" /> : <Plus className="h-4 w-4" />}
            Create
          </button>
        </div>
      </div>

      {/* Profile list */}
      <div className="bg-gray-900 rounded-xl border border-gray-800 p-4">
        <h2 className="text-lg font-semibold text-white mb-4">Your Profiles</h2>
        {profiles.length === 0 ? (
          <p className="text-gray-500">No profiles yet. Create one to get started.</p>
        ) : (
          <div className="space-y-3">
            {profiles.map((profile) => (
              <div
                key={profile.id}
                className={`flex items-center justify-between p-4 rounded-lg border ${
                  profile.is_active
                    ? 'bg-green-900/20 border-green-700'
                    : 'bg-gray-800 border-gray-700'
                }`}
              >
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <h3 className="font-semibold text-white">{profile.name}</h3>
                    {profile.is_active && (
                      <span className="px-2 py-0.5 text-xs bg-green-700 text-green-200 rounded">
                        Active
                      </span>
                    )}
                  </div>
                  {profile.description && (
                    <p className="text-sm text-gray-400 mt-1">{profile.description}</p>
                  )}
                  <p className="text-xs text-gray-500 mt-1">
                    Created: {new Date(profile.created_at).toLocaleDateString()}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  {!profile.is_active && (
                    <button
                      onClick={() => handleActivate(profile.id)}
                      disabled={saving}
                      className="flex items-center gap-2 bg-green-600 hover:bg-green-700 disabled:opacity-50 text-white px-3 py-1.5 rounded-lg transition-colors"
                    >
                      <Check className="h-4 w-4" />
                      Activate
                    </button>
                  )}
                  <button
                    onClick={() => handleDelete(profile.id)}
                    disabled={saving || profile.is_active}
                    className="p-2 hover:bg-red-900/30 text-gray-400 hover:text-red-400 rounded-lg transition-colors disabled:opacity-30"
                    title={profile.is_active ? 'Cannot delete active profile' : 'Delete profile'}
                  >
                    <Trash2 className="h-4 w-4" />
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
