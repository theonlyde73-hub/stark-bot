import { useState, useEffect } from 'react';
import {
  Clock,
  Plus,
  Trash2,
  Play,
  Pause,
  RefreshCw,
  Timer,
  ChevronDown,
  ChevronUp,
  CheckCircle,
  XCircle,
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
  CronJobInfo,
  CronJobRunInfo,
} from '@/lib/api';

export default function Scheduling() {
  const [cronJobs, setCronJobs] = useState<CronJobInfo[]>([]);
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
      const jobs = await getCronJobs();
      setCronJobs(jobs);
    } catch (err) {
      setError('Failed to load cron jobs');
    } finally {
      setIsLoading(false);
    }
  };

  if (isLoading) {
    return (
      <div className="p-8 flex items-center justify-center">
        <div className="flex items-center gap-3">
          <div className="w-6 h-6 border-2 border-stark-500 border-t-transparent rounded-full animate-spin" />
          <span className="text-slate-400">Loading cron jobs...</span>
        </div>
      </div>
    );
  }

  return (
    <div className="p-8">
      <div className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-2xl font-bold text-white mb-2">Cron Jobs</h1>
          <p className="text-slate-400">Schedule automated tasks</p>
        </div>
        {!showCreateForm && (
          <Button onClick={() => setShowCreateForm(true)}>
            <Plus className="w-4 h-4 mr-2" />
            Create Cron Job
          </Button>
        )}
      </div>

      {error && (
        <div className="mb-6 bg-red-500/20 border border-red-500/50 text-red-400 px-4 py-3 rounded-lg">
          {error}
        </div>
      )}

      <CronJobsTab
        jobs={cronJobs}
        setJobs={setCronJobs}
        showCreateForm={showCreateForm}
        setShowCreateForm={setShowCreateForm}
        setError={setError}
      />
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

  // Interval helper state (for 'every' schedule type)
  const [intervalValue, setIntervalValue] = useState(1);
  const [intervalUnit, setIntervalUnit] = useState<'seconds' | 'minutes' | 'hours'>('hours');

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsCreating(true);
    setError(null);

    // Compute schedule_value for 'every' type from intervalValue and intervalUnit
    let scheduleValue = formData.schedule_value;
    if (formData.schedule_type === 'every') {
      const multipliers = { seconds: 1000, minutes: 60000, hours: 3600000 };
      scheduleValue = String(intervalValue * multipliers[intervalUnit]);
    }

    try {
      const newJob = await createCronJob({
        ...formData,
        schedule_value: scheduleValue,
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
      setIntervalValue(1);
      setIntervalUnit('hours');
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
      {showCreateForm && (
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

              {formData.schedule_type === 'every' ? (
                <div>
                  <label className="block text-sm font-medium text-slate-300 mb-2">
                    Interval
                  </label>
                  <div className="flex gap-2">
                    <input
                      type="number"
                      min="1"
                      value={intervalValue}
                      onChange={(e) => setIntervalValue(parseInt(e.target.value) || 1)}
                      className="flex-1 px-4 py-2 bg-slate-700 border border-slate-600 rounded-lg text-white focus:ring-2 focus:ring-stark-500 focus:border-transparent"
                      required
                    />
                    <select
                      value={intervalUnit}
                      onChange={(e) => setIntervalUnit(e.target.value as 'seconds' | 'minutes' | 'hours')}
                      className="px-4 py-2 bg-slate-700 border border-slate-600 rounded-lg text-white focus:ring-2 focus:ring-stark-500 focus:border-transparent"
                    >
                      <option value="seconds">Seconds</option>
                      <option value="minutes">Minutes</option>
                      <option value="hours">Hours</option>
                    </select>
                  </div>
                </div>
              ) : (
                <Input
                  label={formData.schedule_type === 'cron' ? 'Cron Expression' : 'Run At (ISO date)'}
                  value={formData.schedule_value}
                  onChange={(e) => setFormData({ ...formData, schedule_value: e.target.value })}
                  placeholder={formData.schedule_type === 'cron' ? '0 0 * * *' : '2024-12-31T12:00:00Z'}
                  required
                />
              )}

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
