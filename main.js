const { app, BrowserWindow, dialog } = require('electron')
const { fork } = require('node:child_process')
const { createServer } = require('node:net')
const path = require('node:path')

let mainWindow = null
let serverProcess = null

function findFreePort() {
  return new Promise((resolve, reject) => {
    const server = createServer()
    server.listen(0, '127.0.0.1', () => {
      const port = server.address().port
      server.close((err) => {
        if (err) {
          reject(err)
        } else {
          resolve(port)
        }
      })
    })
    server.on('error', reject)
  })
}

function createMainWindow(port) {
  mainWindow = new BrowserWindow({
    width: 1200,
    height: 800,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
    },
  })

  mainWindow.loadURL(`http://127.0.0.1:${port}`)

  mainWindow.on('closed', () => {
    mainWindow = null
  })
}

async function startServer() {
  const port = await findFreePort()
  const serverPath = path.join(__dirname, 'server', 'server.js')

  return new Promise((resolve, reject) => {
    serverProcess = fork(serverPath, [], {
      env: { ...process.env, PORT: String(port) },
      stdio: ['ignore', 'pipe', 'pipe', 'ipc'],
      execArgv: ['--experimental-sqlite'],
    })

    let ready = false

    serverProcess.stdout.on('data', (data) => {
      const msg = data.toString()
      process.stdout.write(msg)
      if (!ready && msg.includes('running at')) {
        ready = true
        resolve(port)
      }
    })

    serverProcess.stderr.on('data', (data) => {
      process.stderr.write(data)
    })

    serverProcess.on('error', (err) => {
      reject(err)
    })

    serverProcess.on('exit', (code) => {
      if (code !== 0 && code !== null) {
        dialog.showErrorBox('Server Crashed', `Server process exited with code ${code}`)
        app.quit()
      }
    })

    // Fallback: if server doesn't log startup message within 5 seconds, assume it's ready
    setTimeout(() => {
      if (!ready) {
        ready = true
        resolve(port)
      }
    }, 5000)
  })
}

app.whenReady().then(async () => {
  try {
    const port = await startServer()
    createMainWindow(port)
  } catch (err) {
    dialog.showErrorBox('Startup Error', err.message)
    process.exit(1)
  }
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit()
  }
})

app.on('before-quit', () => {
  if (serverProcess) {
    serverProcess.kill()
    serverProcess = null
  }
})

app.on('will-quit', () => {
  if (serverProcess) {
    serverProcess.kill()
    serverProcess = null
  }
})