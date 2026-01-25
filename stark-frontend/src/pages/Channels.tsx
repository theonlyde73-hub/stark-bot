import { useState, useEffect } from 'react';
import { MessageSquare, Hash, Plus, Play, Square, Trash2, Save } from 'lucide-react';
import Card, { CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import Button from '@/components/ui/Button';
import Input from '@/components/ui/Input';
import {
  getChannels,
  createChannel,
  updateChannel,
  deleteChannel,
  startChannel,
  stopChannel,
  ChannelInfo,
} from '@/lib/api';

const CHANNEL_TYPES = [
  { value: 'telegram', label: 'Telegram', icon: MessageSquare, color: 'blue' },
  { value: 'slack', label: 'Slack', icon: Hash, color: 'purple' },
];

interface ChannelFormData {
  channel_type: string;
  name: string;
  bot_token: string;
  app_token: string;
}

const emptyForm: ChannelFormData = {
  channel_type: 'telegram',
  name: '',
  bot_token: '',
  app_token: '',
};

export default function Channels() {
  const [channels, setChannels] = useState<ChannelInfo[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showAddForm, setShowAddForm] = useState(false);
  const [newChannel, setNewChannel] = useState<ChannelFormData>(emptyForm);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editForm, setEditForm] = useState<ChannelFormData>(emptyForm);
  const [actionLoading, setActionLoading] = useState<number | null>(null);

  const fetchChannels = async () => {
    try {
      const data = await getChannels();
      setChannels(data);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load channels');
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchChannels();
  }, []);

  const handleCreate = async () => {
    if (!newChannel.name.trim() || !newChannel.bot_token.trim()) {
      setError('Name and bot token are required');
      return;
    }

    if (newChannel.channel_type === 'slack' && !newChannel.app_token.trim()) {
      setError('Slack requires an app token');
      return;
    }

    setActionLoading(-1);
    try {
      await createChannel({
        channel_type: newChannel.channel_type,
        name: newChannel.name,
        bot_token: newChannel.bot_token,
        app_token: newChannel.channel_type === 'slack' ? newChannel.app_token : undefined,
      });
      setNewChannel(emptyForm);
      setShowAddForm(false);
      await fetchChannels();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to create channel');
    } finally {
      setActionLoading(null);
    }
  };

  const handleUpdate = async (id: number) => {
    setActionLoading(id);
    try {
      await updateChannel(id, {
        name: editForm.name || undefined,
        bot_token: editForm.bot_token || undefined,
        app_token: editForm.app_token || undefined,
      });
      setEditingId(null);
      await fetchChannels();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to update channel');
    } finally {
      setActionLoading(null);
    }
  };

  const handleDelete = async (id: number) => {
    if (!confirm('Are you sure you want to delete this channel?')) return;

    setActionLoading(id);
    try {
      await deleteChannel(id);
      await fetchChannels();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to delete channel');
    } finally {
      setActionLoading(null);
    }
  };

  const handleStart = async (id: number) => {
    setActionLoading(id);
    try {
      await startChannel(id);
      await fetchChannels();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to start channel');
    } finally {
      setActionLoading(null);
    }
  };

  const handleStop = async (id: number) => {
    setActionLoading(id);
    try {
      await stopChannel(id);
      await fetchChannels();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to stop channel');
    } finally {
      setActionLoading(null);
    }
  };

  const startEditing = (channel: ChannelInfo) => {
    setEditingId(channel.id);
    setEditForm({
      channel_type: channel.channel_type,
      name: channel.name,
      bot_token: channel.bot_token,
      app_token: channel.app_token || '',
    });
  };

  const getChannelIcon = (type: string) => {
    const channelType = CHANNEL_TYPES.find(c => c.value === type);
    if (!channelType) return { Icon: Hash, color: 'gray' };
    return { Icon: channelType.icon, color: channelType.color };
  };

  if (isLoading) {
    return (
      <div className="p-8 flex items-center justify-center">
        <div className="flex items-center gap-3">
          <div className="w-6 h-6 border-2 border-stark-500 border-t-transparent rounded-full animate-spin" />
          <span className="text-slate-400">Loading channels...</span>
        </div>
      </div>
    );
  }

  return (
    <div className="p-8">
      <div className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white mb-2">Channels</h1>
          <p className="text-slate-400">Configure messaging platform integrations</p>
        </div>
        <Button onClick={() => setShowAddForm(!showAddForm)}>
          <Plus className="w-4 h-4 mr-2" />
          Add Channel
        </Button>
      </div>

      {error && (
        <div className="mb-6 bg-red-500/20 border border-red-500/50 text-red-400 px-4 py-3 rounded-lg flex items-center justify-between">
          <span>{error}</span>
          <button onClick={() => setError(null)} className="text-red-400 hover:text-red-300">
            &times;
          </button>
        </div>
      )}

      {/* Add Channel Form */}
      {showAddForm && (
        <Card className="mb-6">
          <CardHeader>
            <CardTitle>Add New Channel</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">Channel Type</label>
                <select
                  value={newChannel.channel_type}
                  onChange={(e) => setNewChannel({ ...newChannel, channel_type: e.target.value })}
                  className="w-full px-3 py-2 bg-slate-800 border border-slate-700 rounded-lg text-white focus:ring-2 focus:ring-stark-500 focus:border-transparent"
                >
                  {CHANNEL_TYPES.map(type => (
                    <option key={type.value} value={type.value}>{type.label}</option>
                  ))}
                </select>
              </div>
              <Input
                label="Name"
                value={newChannel.name}
                onChange={(e) => setNewChannel({ ...newChannel, name: e.target.value })}
                placeholder="My Telegram Bot"
              />
              <Input
                label="Bot Token"
                value={newChannel.bot_token}
                onChange={(e) => setNewChannel({ ...newChannel, bot_token: e.target.value })}
                placeholder={newChannel.channel_type === 'telegram' ? '123456:ABC-DEF...' : 'xoxb-...'}
              />
              {newChannel.channel_type === 'slack' && (
                <Input
                  label="App Token (for Socket Mode)"
                  value={newChannel.app_token}
                  onChange={(e) => setNewChannel({ ...newChannel, app_token: e.target.value })}
                  placeholder="xapp-..."
                />
              )}
              <div className="flex gap-2 justify-end">
                <Button variant="secondary" onClick={() => setShowAddForm(false)}>
                  Cancel
                </Button>
                <Button onClick={handleCreate} disabled={actionLoading === -1}>
                  {actionLoading === -1 ? 'Creating...' : 'Create Channel'}
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Channel List */}
      <div className="grid gap-6">
        {channels.length === 0 ? (
          <Card>
            <CardContent className="py-12 text-center">
              <p className="text-slate-400">No channels configured yet. Click "Add Channel" to get started.</p>
            </CardContent>
          </Card>
        ) : (
          channels.map((channel) => {
            const { Icon, color } = getChannelIcon(channel.channel_type);
            const isEditing = editingId === channel.id;
            const isActionLoading = actionLoading === channel.id;

            return (
              <Card key={channel.id}>
                <CardHeader>
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <div className={`p-2 bg-${color}-500/20 rounded-lg`}>
                        <Icon className={`w-5 h-5 text-${color}-400`} />
                      </div>
                      <div>
                        <CardTitle>{channel.name}</CardTitle>
                        <span className="text-sm text-slate-400 capitalize">{channel.channel_type}</span>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className={`px-2 py-1 rounded text-xs font-medium ${
                        channel.running
                          ? 'bg-green-500/20 text-green-400'
                          : 'bg-slate-700 text-slate-400'
                      }`}>
                        {channel.running ? 'Running' : 'Stopped'}
                      </span>
                      {channel.running ? (
                        <Button
                          variant="secondary"
                          size="sm"
                          onClick={() => handleStop(channel.id)}
                          disabled={isActionLoading}
                        >
                          <Square className="w-4 h-4 mr-1" />
                          Stop
                        </Button>
                      ) : (
                        <Button
                          variant="secondary"
                          size="sm"
                          onClick={() => handleStart(channel.id)}
                          disabled={isActionLoading}
                        >
                          <Play className="w-4 h-4 mr-1" />
                          Start
                        </Button>
                      )}
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={() => handleDelete(channel.id)}
                        disabled={isActionLoading}
                        className="text-red-400 hover:text-red-300"
                      >
                        <Trash2 className="w-4 h-4" />
                      </Button>
                    </div>
                  </div>
                </CardHeader>
                <CardContent>
                  {isEditing ? (
                    <div className="space-y-4">
                      <Input
                        label="Name"
                        value={editForm.name}
                        onChange={(e) => setEditForm({ ...editForm, name: e.target.value })}
                      />
                      <Input
                        label="Bot Token"
                        value={editForm.bot_token}
                        onChange={(e) => setEditForm({ ...editForm, bot_token: e.target.value })}
                      />
                      {channel.channel_type === 'slack' && (
                        <Input
                          label="App Token"
                          value={editForm.app_token}
                          onChange={(e) => setEditForm({ ...editForm, app_token: e.target.value })}
                        />
                      )}
                      <div className="flex gap-2 justify-end">
                        <Button variant="secondary" onClick={() => setEditingId(null)}>
                          Cancel
                        </Button>
                        <Button onClick={() => handleUpdate(channel.id)} disabled={isActionLoading}>
                          <Save className="w-4 h-4 mr-1" />
                          Save
                        </Button>
                      </div>
                    </div>
                  ) : (
                    <div className="space-y-3">
                      <div>
                        <label className="block text-sm font-medium text-slate-400 mb-1">Bot Token</label>
                        <code className="block px-3 py-2 bg-slate-800 rounded text-sm text-slate-300 font-mono break-all">
                          {channel.bot_token}
                        </code>
                      </div>
                      {channel.app_token && (
                        <div>
                          <label className="block text-sm font-medium text-slate-400 mb-1">App Token</label>
                          <code className="block px-3 py-2 bg-slate-800 rounded text-sm text-slate-300 font-mono break-all">
                            {channel.app_token}
                          </code>
                        </div>
                      )}
                      <div className="flex justify-end">
                        <Button variant="secondary" size="sm" onClick={() => startEditing(channel)}>
                          Edit
                        </Button>
                      </div>
                    </div>
                  )}
                </CardContent>
              </Card>
            );
          })
        )}
      </div>
    </div>
  );
}
