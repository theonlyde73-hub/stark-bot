import { useState, useEffect } from 'react';
import {
  Clock,
  Plus,
  Trash2,
  Play,
  Pause,
  RefreshCw,
  Timer,
  Heart,
  ChevronDown,
  ChevronUp,
  CheckCircle,
  XCircle,
  AlertCircle,
} from 'lucide-react';
import Card, { CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import Button from '@/components/ui/Button';
import Input from '@/components/ui/Input';
import {
  getCronJobs,
  createCronJob,
  deleteCronJob,
  runCronJobNow,
  pauseCronJob,
  resumeCronJob,
  getCronJobRuns,
  getHeartbeatConfig,
  updateHeartbeatConfig,
  CronJobInfo,
  CronJobRunInfo,
  HeartbeatConfigInfo,
} from '@/lib/api';

type TabType = 'cron' | 'heartbeat';

export default function Scheduling() {
  const [activeTab, setActiveTab] = useState<TabType>('cron');
  const [cronJobs, setCronJobs] = useState<CronJobInfo[]>([]);
  const [heartbeatConfig, setHeartbeatConfig] = useState<HeartbeatConfigInfo | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showCreateForm, setShowCreateForm] = useState(false);

  useEffect(() => {
    loadData();
  }, []);

  const loadData = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const [jobs, hbConfig] = await Promise.all([
        getCronJobs(),
        getHeartbeatConfig(),
      ]);
      setCronJobs(jobs);
      setHeartbeatConfig(hbConfig);
    } catch (err) {
      setError('Failed to load scheduling data');
    } finally {
      setIsLoading(false);
    }
  };

  if (isLoading) {
    return (
      <div className="p-8 flex items-center justify-center">
        <div className="flex items-center gap-3">
          <div className="w-6 h-6 border-2 border-stark-500 border-t-transparent rounded-full animate-spin" />
          <span className="text-slate-400">Loading scheduling...</span>
        </div>
      </div>
    );
  }

  return (
    <div className="p-8">
      <div className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-2xl font-bold text-white mb-2">Scheduling</h1>
          <p className="text-slate-400">Manage cron jobs and heartbeat automation</p>
        </div>
      </div>

      {error && (
        <div className="mb-6 bg-red-500/20 border border-red-500/50 text-red-400 px-4 py-3 rounded-lg">
          {error}
        </div>
      )}

      {/* Tabs */}
      <div className="flex gap-2 mb-6">
        <button
          onClick={() => setActiveTab('cron')}
          className={`px-4 py-2 rounded-lg font-medium transition-colors flex items-center gap-2 ${
            activeTab === 'cron'
              ? 'bg-stark-500 text-white'
              : 'bg-slate-700 text-slate-300 hover:bg-slate-600'
          }`}
        >
          <Clock className="w-4 h-4" />
          Cron Jobs
          <span className="ml-1 px-2 py-0.5 text-xs rounded-full bg-slate-800">
            {cronJobs.length}
          </span>
        </button>
        <button
          onClick={() => setActiveTab('heartbeat')}
          className={`px-4 py-2 rounded-lg font-medium transition-colors flex items-center gap-2 ${
            activeTab === 'heartbeat'
              ? 'bg-stark-500 text-white'
              : 'bg-slate-700 text-slate-300 hover:bg-slate-600'
          }`}
        >
          <Heart className="w-4 h-4" />
          Heartbeat
        </button>
      </div>

      {activeTab === 'cron' ? (
        <CronJobsTab
          jobs={cronJobs}
          setJobs={setCronJobs}
          showCreateForm={showCreateForm}
          setShowCreateForm={setShowCreateForm}
          setError={setError}
        />
      ) : (
        <HeartbeatTab
          config={heartbeatConfig}
          setConfig={setHeartbeatConfig}
          setError={setError}
        />
      )}
    </div>
  );
}

interface CronJobsTabProps {
  jobs: CronJobInfo[];
  setJobs: React.Dispatch<React.SetStateAction<CronJobInfo[]>>;
  showCreateForm: boolean;
  setShowCreateForm: React.Dispatch<React.SetStateAction<boolean>>;
  setError: React.Dispatch<React.SetStateAction<string | null>>;
}

function CronJobsTab({ jobs, setJobs, showCreateForm, setShowCreateForm, setError }: CronJobsTabProps) {
  const [isCreating, setIsCreating] = useState(false);
  const [expandedJob, setExpandedJob] = useState<number | null>(null);
  const [jobRuns, setJobRuns] = useState<Record<number, CronJobRunInfo[]>>({});

  // Form state
  const [formData, setFormData] = useState({
    name: '',
    description: '',
    schedule_type: 'every',
    schedule_value: '',
    session_mode: 'main',
    message: '',
  });

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsCreating(true);
    setError(null);

    try {
      const newJob = await createCronJob({
        ...formData,
        deliver: false,
      });
      setJobs((prev) => [...prev, newJob]);
      setShowCreateForm(false);
      setFormData({
        name: '',
        description: '',
        schedule_type: 'every',
        schedule_value: '',
        session_mode: 'main',
        message: '',
      });
    } catch (err) {
      setError('Failed to create cron job');
    } finally {
      setIsCreating(false);
    }
  };

  const handleDelete = async (id: number, name: string) => {
    if (!confirm(`Are you sure you want to delete the job "${name}"?`)) return;

    try {
      await deleteCronJob(id);
      setJobs((prev) => prev.filter((j) => j.id !== id));
    } catch (err) {
      setError('Failed to delete cron job');
    }
  };

  const handleRunNow = async (id: number) => {
    try {
      await runCronJobNow(id);
      setError(null);
      // Reload jobs to get updated last_run_at
      const updatedJobs = await getCronJobs();
      setJobs(updatedJobs);
    } catch (err) {
      setError('Failed to run cron job');
    }
  };

  const handleTogglePause = async (job: CronJobInfo) => {
    try {
      let updatedJob: CronJobInfo;
      if (job.status === 'paused') {
        updatedJob = await resumeCronJob(job.id);
      } else {
        updatedJob = await pauseCronJob(job.id);
      }
      setJobs((prev) => prev.map((j) => (j.id === job.id ? updatedJob : j)));
    } catch (err) {
      setError('Failed to toggle job status');
    }
  };

  const handleExpand = async (id: number) => {
    if (expandedJob === id) {
      setExpandedJob(null);
    } else {
      setExpandedJob(id);
      if (!jobRuns[id]) {
        try {
          const runs = await getCronJobRuns(id, 5);
          setJobRuns((prev) => ({ ...prev, [id]: runs }));
        } catch (err) {
          console.error('Failed to load job runs');
        }
      }
    }
  };

  const getScheduleDisplay = (job: CronJobInfo) => {
    switch (job.schedule_type) {
      case 'at':
        return `At ${job.schedule_value}`;
      case 'every':
        const ms = parseInt(job.schedule_value);
        if (ms >= 3600000) return `Every ${Math.round(ms / 3600000)}h`;
        if (ms >= 60000) return `Every ${Math.round(ms / 60000)}m`;
        return `Every ${Math.round(ms / 1000)}s`;
      case 'cron':
        return job.schedule_value;
      default:
        return job.schedule_value;
    }
  };

  const getStatusBadge = (status: string) => {
    switch (status) {
      case 'active':
        return <span className="px-2 py-1 text-xs rounded bg-green-500/20 text-green-400">Active</span>;
      case 'paused':
        return <span className="px-2 py-1 text-xs rounded bg-yellow-500/20 text-yellow-400">Paused</span>;
      case 'completed':
        return <span className="px-2 py-1 text-xs rounded bg-blue-500/20 text-blue-400">Completed</span>;
      case 'failed':
        return <span className="px-2 py-1 text-xs rounded bg-red-500/20 text-red-400">Failed</span>;
      default:
        return <span className="px-2 py-1 text-xs rounded bg-slate-700 text-slate-400">{status}</span>;
    }
  };

  return (
    <div className="space-y-4">
      {/* Create Form */}
      {showCreateForm ? (
        <Card>
          <CardHeader>
            <CardTitle>Create Cron Job</CardTitle>
          </CardHeader>
          <CardContent>
            <form onSubmit={handleCreate} className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <Input
                  label="Name"
                  value={formData.name}
                  onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                  placeholder="my-job"
                  required
                />
                <div>
                  <label className="block text-sm font-medium text-slate-300 mb-2">
                    Schedule Type
                  </label>
                  <select
                    value={formData.schedule_type}
                    onChange={(e) => setFormData({ ...formData, schedule_type: e.target.value })}
                    className="w-full px-4 py-2 bg-slate-700 border border-slate-600 rounded-lg text-white focus:ring-2 focus:ring-stark-500 focus:border-transparent"
                  >
                    <option value="every">Every (interval)</option>
                    <option value="at">At (one-time)</option>
                    <option value="cron">Cron Expression</option>
                  </select>
                </div>
              </div>

              <Input
                label={formData.schedule_type === 'cron' ? 'Cron Expression' : formData.schedule_type === 'at' ? 'Run At (ISO date)' : 'Interval (ms)'}
                value={formData.schedule_value}
                onChange={(e) => setFormData({ ...formData, schedule_value: e.target.value })}
                placeholder={formData.schedule_type === 'cron' ? '0 0 * * *' : formData.schedule_type === 'at' ? '2024-12-31T12:00:00Z' : '3600000'}
                required
              />

              <Input
                label="Description"
                value={formData.description}
                onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                placeholder="Optional description"
              />

              <Input
                label="Message / Task"
                value={formData.message}
                onChange={(e) => setFormData({ ...formData, message: e.target.value })}
                placeholder="What should the agent do?"
                required
              />

              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">
                  Session Mode
                </label>
                <select
                  value={formData.session_mode}
                  onChange={(e) => setFormData({ ...formData, session_mode: e.target.value })}
                  className="w-full px-4 py-2 bg-slate-700 border border-slate-600 rounded-lg text-white focus:ring-2 focus:ring-stark-500 focus:border-transparent"
                >
                  <option value="main">Main (shared context)</option>
                  <option value="isolated">Isolated (fresh context)</option>
                </select>
              </div>

              <div className="flex gap-2">
                <Button type="submit" isLoading={isCreating}>
                  <Plus className="w-4 h-4 mr-2" />
                  Create Job
                </Button>
                <Button type="button" variant="secondary" onClick={() => setShowCreateForm(false)}>
                  Cancel
                </Button>
              </div>
            </form>
          </CardContent>
        </Card>
      ) : (
        <Button onClick={() => setShowCreateForm(true)}>
          <Plus className="w-4 h-4 mr-2" />
          Create Cron Job
        </Button>
      )}

      {/* Job List */}
      {jobs.length > 0 ? (
        <div className="space-y-3">
          {jobs.map((job) => (
            <Card key={job.id}>
              <CardContent className="p-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-4">
                    <div className="p-3 bg-stark-500/20 rounded-lg">
                      <Timer className="w-6 h-6 text-stark-400" />
                    </div>
                    <div>
                      <div className="flex items-center gap-2">
                        <h3 className="font-semibold text-white">{job.name}</h3>
                        {getStatusBadge(job.status)}
                        <span className="text-xs px-2 py-0.5 bg-slate-700 text-slate-400 rounded">
                          {getScheduleDisplay(job)}
                        </span>
                      </div>
                      {job.description && (
                        <p className="text-sm text-slate-400 mt-1">{job.description}</p>
                      )}
                      <div className="flex items-center gap-4 mt-2 text-xs text-slate-500">
                        {job.last_run_at && (
                          <span>Last: {new Date(job.last_run_at).toLocaleString()}</span>
                        )}
                        {job.next_run_at && (
                          <span>Next: {new Date(job.next_run_at).toLocaleString()}</span>
                        )}
                      </div>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleRunNow(job.id)}
                      title="Run now"
                    >
                      <Play className="w-4 h-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleTogglePause(job)}
                      title={job.status === 'paused' ? 'Resume' : 'Pause'}
                    >
                      {job.status === 'paused' ? (
                        <RefreshCw className="w-4 h-4" />
                      ) : (
                        <Pause className="w-4 h-4" />
                      )}
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleExpand(job.id)}
                    >
                      {expandedJob === job.id ? (
                        <ChevronUp className="w-4 h-4" />
                      ) : (
                        <ChevronDown className="w-4 h-4" />
                      )}
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleDelete(job.id, job.name)}
                      className="text-red-400 hover:text-red-300 hover:bg-red-500/20"
                    >
                      <Trash2 className="w-4 h-4" />
                    </Button>
                  </div>
                </div>

                {/* Expanded Details */}
                {expandedJob === job.id && (
                  <div className="mt-4 pt-4 border-t border-slate-700">
                    <div className="grid grid-cols-2 gap-4 mb-4">
                      <div>
                        <p className="text-xs text-slate-500">Message</p>
                        <p className="text-sm text-slate-300">{job.message || '(none)'}</p>
                      </div>
                      <div>
                        <p className="text-xs text-slate-500">Session Mode</p>
                        <p className="text-sm text-slate-300">{job.session_mode}</p>
                      </div>
                    </div>

                    {jobRuns[job.id] && jobRuns[job.id].length > 0 && (
                      <div>
                        <p className="text-xs text-slate-500 mb-2">Recent Runs</p>
                        <div className="space-y-2">
                          {jobRuns[job.id].map((run) => (
                            <div
                              key={run.id}
                              className="flex items-center gap-3 p-2 bg-slate-800 rounded"
                            >
                              {run.success ? (
                                <CheckCircle className="w-4 h-4 text-green-400" />
                              ) : (
                                <XCircle className="w-4 h-4 text-red-400" />
                              )}
                              <span className="text-xs text-slate-400">
                                {new Date(run.started_at).toLocaleString()}
                              </span>
                              {run.duration_ms && (
                                <span className="text-xs text-slate-500">
                                  {run.duration_ms}ms
                                </span>
                              )}
                              {run.error && (
                                <span className="text-xs text-red-400 truncate flex-1">
                                  {run.error}
                                </span>
                              )}
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                )}
              </CardContent>
            </Card>
          ))}
        </div>
      ) : (
        <Card>
          <CardContent className="text-center py-12">
            <Clock className="w-12 h-12 text-slate-600 mx-auto mb-4" />
            <p className="text-slate-400 mb-4">No cron jobs configured</p>
            <Button variant="secondary" onClick={() => setShowCreateForm(true)}>
              <Plus className="w-4 h-4 mr-2" />
              Create Your First Job
            </Button>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

interface HeartbeatTabProps {
  config: HeartbeatConfigInfo | null;
  setConfig: React.Dispatch<React.SetStateAction<HeartbeatConfigInfo | null>>;
  setError: React.Dispatch<React.SetStateAction<string | null>>;
}

function HeartbeatTab({ config, setConfig, setError }: HeartbeatTabProps) {
  const [isSaving, setIsSaving] = useState(false);
  const [formData, setFormData] = useState({
    interval_minutes: config?.interval_minutes || 60,
    target: config?.target || '',
    active_hours_start: config?.active_hours_start || '09:00',
    active_hours_end: config?.active_hours_end || '17:00',
    active_days: config?.active_days || 'mon,tue,wed,thu,fri',
    enabled: config?.enabled || false,
  });

  useEffect(() => {
    if (config) {
      setFormData({
        interval_minutes: config.interval_minutes,
        target: config.target || '',
        active_hours_start: config.active_hours_start || '09:00',
        active_hours_end: config.active_hours_end || '17:00',
        active_days: config.active_days || 'mon,tue,wed,thu,fri',
        enabled: config.enabled,
      });
    }
  }, [config]);

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSaving(true);
    setError(null);

    try {
      const updated = await updateHeartbeatConfig(formData);
      setConfig(updated);
    } catch (err) {
      setError('Failed to update heartbeat config');
    } finally {
      setIsSaving(false);
    }
  };

  const toggleEnabled = async () => {
    setIsSaving(true);
    try {
      const updated = await updateHeartbeatConfig({
        enabled: !formData.enabled,
      });
      setConfig(updated);
      setFormData((prev) => ({ ...prev, enabled: !prev.enabled }));
    } catch (err) {
      setError('Failed to toggle heartbeat');
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="flex items-center gap-2">
            <Heart className="w-5 h-5 text-red-400" />
            Heartbeat Configuration
          </CardTitle>
          <button
            onClick={toggleEnabled}
            disabled={isSaving}
            className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
              formData.enabled ? 'bg-stark-500' : 'bg-slate-600'
            }`}
          >
            <span
              className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                formData.enabled ? 'translate-x-6' : 'translate-x-1'
              }`}
            />
          </button>
        </div>
      </CardHeader>
      <CardContent>
        <form onSubmit={handleSave} className="space-y-6">
          <div className="bg-slate-800/50 rounded-lg p-4">
            <div className="flex items-start gap-3">
              <AlertCircle className="w-5 h-5 text-stark-400 mt-0.5" />
              <div>
                <p className="text-sm text-slate-300">
                  Heartbeat sends periodic check-in messages to your agent, prompting it to
                  review pending tasks, notifications, and scheduled items.
                </p>
              </div>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-slate-300 mb-2">
                Interval (minutes)
              </label>
              <input
                type="number"
                min="1"
                value={formData.interval_minutes}
                onChange={(e) => setFormData({ ...formData, interval_minutes: parseInt(e.target.value) || 60 })}
                className="w-full px-4 py-2 bg-slate-700 border border-slate-600 rounded-lg text-white focus:ring-2 focus:ring-stark-500 focus:border-transparent"
              />
            </div>
            <Input
              label="Target (optional)"
              value={formData.target}
              onChange={(e) => setFormData({ ...formData, target: e.target.value })}
              placeholder="Channel or identity to target"
            />
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-slate-300 mb-2">
                Active Hours Start
              </label>
              <input
                type="time"
                value={formData.active_hours_start}
                onChange={(e) => setFormData({ ...formData, active_hours_start: e.target.value })}
                className="w-full px-4 py-2 bg-slate-700 border border-slate-600 rounded-lg text-white focus:ring-2 focus:ring-stark-500 focus:border-transparent"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-slate-300 mb-2">
                Active Hours End
              </label>
              <input
                type="time"
                value={formData.active_hours_end}
                onChange={(e) => setFormData({ ...formData, active_hours_end: e.target.value })}
                className="w-full px-4 py-2 bg-slate-700 border border-slate-600 rounded-lg text-white focus:ring-2 focus:ring-stark-500 focus:border-transparent"
              />
            </div>
          </div>

          <div>
            <label className="block text-sm font-medium text-slate-300 mb-2">
              Active Days
            </label>
            <div className="flex flex-wrap gap-2">
              {['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun'].map((day) => {
                const isActive = formData.active_days.toLowerCase().includes(day);
                return (
                  <button
                    key={day}
                    type="button"
                    onClick={() => {
                      const days = formData.active_days.split(',').map((d) => d.trim().toLowerCase());
                      const newDays = isActive
                        ? days.filter((d) => d !== day)
                        : [...days, day];
                      setFormData({ ...formData, active_days: newDays.join(',') });
                    }}
                    className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${
                      isActive
                        ? 'bg-stark-500 text-white'
                        : 'bg-slate-700 text-slate-400 hover:bg-slate-600'
                    }`}
                  >
                    {day.charAt(0).toUpperCase() + day.slice(1)}
                  </button>
                );
              })}
            </div>
          </div>

          {config && (
            <div className="grid grid-cols-2 gap-4 text-sm">
              <div>
                <p className="text-slate-500">Last heartbeat</p>
                <p className="text-slate-300">
                  {config.last_beat_at
                    ? new Date(config.last_beat_at).toLocaleString()
                    : 'Never'}
                </p>
              </div>
              <div>
                <p className="text-slate-500">Next heartbeat</p>
                <p className="text-slate-300">
                  {config.next_beat_at
                    ? new Date(config.next_beat_at).toLocaleString()
                    : 'Not scheduled'}
                </p>
              </div>
            </div>
          )}

          <Button type="submit" isLoading={isSaving}>
            Save Configuration
          </Button>
        </form>
      </CardContent>
    </Card>
  );
}
