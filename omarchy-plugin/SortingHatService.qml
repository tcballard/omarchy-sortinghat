import QtQuick
import Quickshell
import Quickshell.Io

Item {
  id: root

  property var shell: null
  property string runtimeState: "checking"
  property string lastError: ""
  property var queue: []
  property var roots: []
  property int awaitingReview: 0
  property var pendingAction: null

  function command(args) {
    var result = ["sortinghat", "--json"]
    for (var i = 0; i < args.length; i++) result.push(String(args[i]))
    return result
  }

  function parse(text) {
    try {
      var payload = JSON.parse(String(text || ""))
      return payload.schema_version === 1 ? payload : null
    } catch (error) {
      return null
    }
  }

  function refresh() {
    if (statusProcess.running || queueProcess.running || actionProcess.running) return
    statusProcess.command = command(["status"])
    statusProcess.running = true
  }

  function run(args, callback) {
    if (actionProcess.running) return false
    pendingAction = callback || null
    actionProcess.command = command(args)
    actionProcess.running = true
    return true
  }

  function addWatchedRoot(path) {
    run(["roots", "add", path, "--watch", "--destination"])
  }

  Process {
    id: statusProcess
    stdout: StdioCollector { id: statusOutput; waitForEnd: true }
    onExited: function(exitCode) {
      var payload = root.parse(statusOutput.text)
      if (exitCode !== 0 || !payload || !payload.ok) {
        root.runtimeState = payload ? payload.state : "runtime_missing"
        root.lastError = payload ? payload.message : "SortingHat runtime is not installed or unavailable"
        root.awaitingReview = 0
        return
      }
      root.runtimeState = payload.state
      root.lastError = ""
      root.awaitingReview = Number(payload.data.awaiting_review || 0)
      queueProcess.command = root.command(["queue", "list"])
      queueProcess.running = true
    }
  }

  Process {
    id: queueProcess
    stdout: StdioCollector { id: queueOutput; waitForEnd: true }
    onExited: function(exitCode) {
      var payload = root.parse(queueOutput.text)
      if (exitCode !== 0 || !payload || !payload.ok) {
        root.runtimeState = payload ? payload.state : "runtime_error"
        root.lastError = payload ? payload.message : "Could not read the review queue"
        return
      }
      root.queue = Array.isArray(payload.data) ? payload.data : []
      rootsProcess.command = root.command(["roots", "list"])
      rootsProcess.running = true
    }
  }

  Process {
    id: rootsProcess
    stdout: StdioCollector { id: rootsOutput; waitForEnd: true }
    onExited: function(exitCode) {
      var payload = root.parse(rootsOutput.text)
      if (exitCode === 0 && payload && payload.ok)
        root.roots = Array.isArray(payload.data) ? payload.data : []
    }
  }

  Process {
    id: actionProcess
    stdout: StdioCollector { id: actionOutput; waitForEnd: true }
    onExited: function(exitCode) {
      var payload = root.parse(actionOutput.text)
      if (exitCode !== 0 || !payload || !payload.ok) {
        root.runtimeState = payload ? payload.state : "runtime_error"
        root.lastError = payload ? payload.message : "The requested action failed safely"
      } else if (root.pendingAction) {
        root.pendingAction(payload)
      }
      root.pendingAction = null
      refreshDelay.restart()
    }
  }

  Timer {
    interval: 2000
    running: true
    repeat: true
    triggeredOnStart: true
    onTriggered: root.refresh()
  }

  Timer {
    id: refreshDelay
    interval: 100
    onTriggered: root.refresh()
  }
}
