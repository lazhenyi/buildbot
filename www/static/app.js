/* Buildbot Dispatcher - API Client */

var API_BASE = '/api/v1';

function apiFetch(path, options) {
    return fetch(API_BASE + path, options)
        .then(function(response) {
            if (!response.ok) {
                return response.json().then(function(err) {
                    throw new Error(err.error || 'Request failed: ' + response.status);
                }).catch(function() {
                    throw new Error('Request failed: ' + response.status);
                });
            }
            return response.json();
        });
}

/* Dispatcher Info */
function loadDispatcherInfo() {
    apiFetch('/info')
        .then(function(data) {
            document.getElementById('version').textContent = data.version || 'v0.1.0';
        })
        .catch(function(err) {
            console.error('Failed to load dispatcher info:', err);
        });
}

/* Dashboard */
function loadDashboard() {
    loadDispatcherInfo();
    loadRecentJobs();
    loadRunners();
    loadProjects();
    loadBuilders();
    loadRecentBuilds();
}

function loadRecentJobs() {
    var el = document.getElementById('recent-jobs-loading');
    var table = document.getElementById('recent-jobs-table');
    var tbody = document.getElementById('recent-jobs-body');

    apiFetch('/jobs?limit=10')
        .then(function(data) {
            el.style.display = 'none';
            table.style.display = 'table';
            if (!data.jobs || data.jobs.length === 0) {
                tbody.innerHTML = '<tr><td colspan="5" class="empty-state">No jobs found</td></tr>';
                return;
            }
            tbody.innerHTML = data.jobs.map(function(job) {
                return '<tr>' +
                    '<td><a href="/jobs/' + job.id + '">' + job.id.substring(0, 8) + '</a></td>' +
                    '<td>' + escHtml(job.name) + '</td>' +
                    '<td><span class="status-badge status-' + job.status + '">' + job.status + '</span></td>' +
                    '<td>' + (job.runner_id ? job.runner_id.substring(0, 8) : '-') + '</td>' +
                    '<td>' + fmtDate(job.created_at) + '</td>' +
                    '</tr>';
            }).join('');
        })
        .catch(function(err) {
            el.textContent = 'Error: ' + err.message;
        });
}

function loadRunners() {
    var el = document.getElementById('runners-loading');
    var table = document.getElementById('runners-table');
    var tbody = document.getElementById('runners-body');

    apiFetch('/runners')
        .then(function(data) {
            el.style.display = 'none';
            table.style.display = 'table';
            if (!data.runners || data.runners.length === 0) {
                tbody.innerHTML = '<tr><td colspan="5" class="empty-state">No runners registered</td></tr>';
                return;
            }
            tbody.innerHTML = data.runners.map(function(runner) {
                var status = runner.last_seen &&
                    (Date.now() - new Date(runner.last_seen).getTime() < 300000)
                    ? 'connected' : 'disconnected';
                return '<tr>' +
                    '<td>' + runner.id.substring(0, 8) + '</td>' +
                    '<td>' + escHtml(runner.name) + '</td>' +
                    '<td>' + fmtLabels(runner.labels) + '</td>' +
                    '<td><span class="status-badge status-' + status + '">' + status + '</span></td>' +
                    '<td>' + fmtDate(runner.last_seen) + '</td>' +
                    '</tr>';
            }).join('');
        })
        .catch(function(err) {
            el.textContent = 'Error: ' + err.message;
        });
}

function loadProjects() {
    var el = document.getElementById('projects-loading');
    var table = document.getElementById('projects-table');
    var tbody = document.getElementById('projects-body');

    apiFetch('/projects')
        .then(function(data) {
            el.style.display = 'none';
            if (!data.projects || data.projects.length === 0) {
                table.style.display = 'none';
                return;
            }
            table.style.display = 'table';
            tbody.innerHTML = data.projects.map(function(p) {
                return '<tr>' +
                    '<td>' + p.id + '</td>' +
                    '<td>' + escHtml(p.name) + '</td>' +
                    '<td>' + escHtml(p.slug || '-') + '</td>' +
                    '<td><code>' + escHtml(p.repository_url || '-') + '</code></td>' +
                    '</tr>';
            }).join('');
        })
        .catch(function(err) {
            el.textContent = 'Error: ' + err.message;
        });
}

function loadBuilders() {
    var el = document.getElementById('builders-loading');
    var table = document.getElementById('builders-table');
    var tbody = document.getElementById('builders-body');

    apiFetch('/builders')
        .then(function(data) {
            el.style.display = 'none';
            if (!data.builders || data.builders.length === 0) {
                table.style.display = 'none';
                return;
            }
            table.style.display = 'table';
            tbody.innerHTML = data.builders.map(function(b) {
                return '<tr>' +
                    '<td>' + b.id + '</td>' +
                    '<td>' + escHtml(b.name) + '</td>' +
                    '<td>' + escHtml(b.project_name || '-') + '</td>' +
                    '</tr>';
            }).join('');
        })
        .catch(function(err) {
            el.textContent = 'Error: ' + err.message;
        });
}

function loadRecentBuilds() {
    var el = document.getElementById('builds-loading');
    var table = document.getElementById('builds-table');
    var tbody = document.getElementById('builds-body');

    apiFetch('/builds?limit=20')
        .then(function(data) {
            el.style.display = 'none';
            if (!data.builds || data.builds.length === 0) {
                table.style.display = 'none';
                return;
            }
            table.style.display = 'table';
            tbody.innerHTML = data.builds.map(function(b) {
                return '<tr>' +
                    '<td>' + b.id + '</td>' +
                    '<td>' + b.number + '</td>' +
                    '<td>' + escHtml(b.builder_name || '-') + '</td>' +
                    '<td><span class="status-badge status-' + b.state_string.toLowerCase() + '">' + b.state_string + '</span></td>' +
                    '<td>' + fmtDate(b.started_at) + '</td>' +
                    '<td>' + fmtDate(b.complete_at) + '</td>' +
                    '</tr>';
            }).join('');
        })
        .catch(function(err) {
            el.textContent = 'Error: ' + err.message;
        });
}

/* Jobs Page */
function loadJobs() {
    var filter = document.getElementById('status-filter') ? document.getElementById('status-filter').value : '';
    var el = document.getElementById('jobs-loading');
    var table = document.getElementById('jobs-table');
    var empty = document.getElementById('jobs-empty');
    var tbody = document.getElementById('jobs-body');

    var path = '/jobs?limit=50';
    if (filter) path += '&status=' + filter;

    apiFetch(path)
        .then(function(data) {
            el.style.display = 'none';
            if (!data.jobs || data.jobs.length === 0) {
                table.style.display = 'none';
                empty.style.display = 'block';
                return;
            }
            table.style.display = 'table';
            empty.style.display = 'none';
            tbody.innerHTML = data.jobs.map(function(job) {
                var actions = '';
                if (job.status === 'pending') {
                    actions = '<a class="action-link" onclick="cancelJobById(\'' + job.id + '\')" href="javascript:void(0)">Cancel</a>';
                }
                return '<tr>' +
                    '<td><a href="/jobs/' + job.id + '">' + job.id.substring(0, 8) + '</a></td>' +
                    '<td>' + escHtml(job.name) + '</td>' +
                    '<td><span class="status-badge status-' + job.status + '">' + job.status + '</span></td>' +
                    '<td>' + (job.runner_id ? job.runner_id.substring(0, 8) : '-') + '</td>' +
                    '<td>' + fmtLabels(job.labels) + '</td>' +
                    '<td>' + fmtDate(job.created_at) + '</td>' +
                    '<td>' + fmtDate(job.started_at) + '</td>' +
                    '<td>' + fmtDate(job.completed_at) + '</td>' +
                    '<td>' + actions + '</td>' +
                    '</tr>';
            }).join('');
        })
        .catch(function(err) {
            el.textContent = 'Error: ' + err.message;
        });
}

function showEnqueueModal() {
    document.getElementById('enqueue-modal').style.display = 'flex';
    document.getElementById('enqueue-result').style.display = 'none';
    document.getElementById('job-name').value = '';
    document.getElementById('job-labels').value = '';
    document.getElementById('job-timeout').value = '3600';
    document.getElementById('job-script').value = '';
}

function closeEnqueueModal() {
    document.getElementById('enqueue-modal').style.display = 'none';
}

function enqueueJob() {
    var name = document.getElementById('job-name').value.trim();
    var labelsStr = document.getElementById('job-labels').value.trim();
    var timeout = parseInt(document.getElementById('job-timeout').value, 10);
    var script = document.getElementById('job-script').value.trim();
    var resultBox = document.getElementById('enqueue-result');

    if (!name) {
        resultBox.className = 'result-box error';
        resultBox.textContent = 'Error: Job name is required';
        resultBox.style.display = 'block';
        return;
    }

    var labels = labelsStr ? labelsStr.split(',').map(function(l) { return l.trim(); }).filter(Boolean) : [];

    var body = {
        name: name,
        labels: labels
    };
    if (timeout) body.timeout_secs = timeout;
    if (script) body.script = script;

    resultBox.className = 'result-box';
    resultBox.textContent = 'Submitting...';
    resultBox.style.display = 'block';

    apiFetch('/jobs/enqueue', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body)
    })
        .then(function(data) {
            resultBox.className = 'result-box success';
            resultBox.textContent = JSON.stringify(data, null, 2);
            setTimeout(function() {
                closeEnqueueModal();
                loadJobs();
            }, 1500);
        })
        .catch(function(err) {
            resultBox.className = 'result-box error';
            resultBox.textContent = 'Error: ' + err.message;
        });
}

function cancelJob() {
    if (typeof JOB_ID !== 'undefined') {
        cancelJobById(JOB_ID);
    }
}

function cancelJobById(jobId) {
    if (!confirm('Cancel job ' + jobId.substring(0, 8) + '?')) return;
    apiFetch('/jobs/' + jobId + '/cancel', { method: 'POST' })
        .then(function(data) {
            alert('Job cancelled');
            loadJobs();
        })
        .catch(function(err) {
            alert('Error: ' + err.message);
        });
}

/* Job Detail Page */
function loadJobDetail() {
    if (typeof JOB_ID === 'undefined') return;
    var el = document.getElementById('job-loading');
    var table = document.getElementById('job-detail-table');
    var tbody = document.getElementById('job-detail-body');
    var jsonEl = document.getElementById('job-json');

    apiFetch('/jobs/' + JOB_ID)
        .then(function(data) {
            el.style.display = 'none';
            table.style.display = 'table';
            var job = data.job || data;

            var fields = [
                ['ID', job.id],
                ['Name', job.name],
                ['Status', job.status],
                ['Runner ID', job.runner_id || '-'],
                ['Labels', (job.labels || []).join(', ') || '-'],
                ['Timeout', job.timeout_secs ? job.timeout_secs + 's' : '-'],
                ['Created At', fmtDate(job.created_at)],
                ['Started At', fmtDate(job.started_at)],
                ['Completed At', fmtDate(job.completed_at)],
                ['Exit Code', job.exit_code !== null && job.exit_code !== undefined ? job.exit_code : '-'],
                ['Script', job.script ? '\n' + job.script : '-']
            ];

            tbody.innerHTML = fields.map(function(f) {
                return '<tr><th>' + escHtml(f[0]) + '</th><td>' + escHtml(String(f[1])).replace(/\n/g, '<br>') + '</td></tr>';
            }).join('');

            jsonEl.textContent = JSON.stringify(data, null, 2);
        })
        .catch(function(err) {
            el.textContent = 'Error: ' + err.message;
        });
}

/* Runners Page */
function loadRunnersOnPage() {
    loadRunners();
}

function showRegisterModal() {
    document.getElementById('register-modal').style.display = 'flex';
    document.getElementById('register-result').style.display = 'none';
    document.getElementById('runner-name').value = '';
    document.getElementById('runner-labels').value = '';
    document.getElementById('runner-arch').value = '';
}

function closeRegisterModal() {
    document.getElementById('register-modal').style.display = 'none';
}

function registerRunner() {
    var name = document.getElementById('runner-name').value.trim();
    var labelsStr = document.getElementById('runner-labels').value.trim();
    var arch = document.getElementById('runner-arch').value.trim();
    var resultBox = document.getElementById('register-result');

    if (!name) {
        resultBox.className = 'result-box error';
        resultBox.textContent = 'Error: Runner name is required';
        resultBox.style.display = 'block';
        return;
    }

    var labels = labelsStr ? labelsStr.split(',').map(function(l) { return l.trim(); }).filter(Boolean) : [];

    var body = { name: name, labels: labels };
    if (arch) body.architecture = arch;

    resultBox.className = 'result-box';
    resultBox.textContent = 'Registering...';
    resultBox.style.display = 'block';

    apiFetch('/runners/register', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body)
    })
        .then(function(data) {
            resultBox.className = 'result-box success';
            resultBox.textContent = JSON.stringify(data, null, 2);
            setTimeout(function() {
                closeRegisterModal();
                loadRunners();
            }, 1500);
        })
        .catch(function(err) {
            resultBox.className = 'result-box error';
            resultBox.textContent = 'Error: ' + err.message;
        });
}

function unregisterRunner(runnerId) {
    if (!confirm('Unregister runner ' + runnerId.substring(0, 8) + '?')) return;
    apiFetch('/runners/' + runnerId + '/unregister', { method: 'POST' })
        .then(function(data) {
            alert('Runner unregistered');
            loadRunners();
        })
        .catch(function(err) {
            alert('Error: ' + err.message);
        });
}

function pollJobs() {
    var runnerId = document.getElementById('poll-runner-id').value.trim();
    var labelsStr = document.getElementById('poll-labels').value.trim();
    var resultEl = document.getElementById('poll-result');

    if (!runnerId) {
        resultEl.textContent = 'Error: Runner ID is required';
        return;
    }

    var labels = labelsStr ? labelsStr.split(',').map(function(l) { return l.trim(); }).filter(Boolean) : [];

    var path = '/jobs/poll?runner_id=' + encodeURIComponent(runnerId);
    if (labels.length > 0) path += '&labels=' + encodeURIComponent(labels.join(','));

    resultEl.textContent = 'Polling...';

    apiFetch(path)
        .then(function(data) {
            resultEl.textContent = JSON.stringify(data, null, 2);
        })
        .catch(function(err) {
            resultEl.textContent = 'Error: ' + err.message;
        });
}

/* Utilities */
function escHtml(str) {
    if (str === null || str === undefined) return '';
    return String(str)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;');
}

function fmtDate(dateStr) {
    if (!dateStr) return '-';
    try {
        var d = new Date(dateStr);
        if (isNaN(d.getTime())) return '-';
        return d.toLocaleString();
    } catch (e) {
        return '-';
    }
}

function fmtLabels(labels) {
    if (!labels || labels.length === 0) return '-';
    return '<div class="labels-cell">' +
        labels.map(function(l) { return '<span class="label-tag">' + escHtml(l) + '</span>'; }).join('') +
        '</div>';
}

function refreshDashboard() {
    loadDashboard();
}
