const http = require('node:http')
const fs = require('node:fs')
const path = require('node:path')
const { buildSnapshot } = require('./dashboard-service')

const HOST = '127.0.0.1'
const PORT = Number(process.env.PORT || 4317)
const PUBLIC_DIR = path.join(__dirname, '..', 'public')

function sendJson(res, statusCode, payload) {
  res.writeHead(statusCode, {
    'Content-Type': 'application/json; charset=utf-8',
    'Cache-Control': 'no-store',
  })
  res.end(JSON.stringify(payload))
}

function sendFile(res, filePath) {
  if (!fs.existsSync(filePath)) {
    res.writeHead(404)
    res.end('Not found')
    return
  }
  const ext = path.extname(filePath)
  const type = ext === '.css' ? 'text/css; charset=utf-8'
    : ext === '.js' ? 'application/javascript; charset=utf-8'
    : 'text/html; charset=utf-8'

  res.writeHead(200, { 'Content-Type': type })
  fs.createReadStream(filePath).pipe(res)
}

const server = http.createServer((req, res) => {
  const url = new URL(req.url, `http://${req.headers.host}`)

  if (url.pathname === '/api/health') {
    sendJson(res, 200, { ok: true })
    return
  }

  if (url.pathname === '/api/dashboard/snapshot') {
    try {
      sendJson(res, 200, buildSnapshot())
    } catch (error) {
      sendJson(res, 500, {
        error: 'snapshot_failed',
        message: error instanceof Error ? error.message : String(error),
      })
    }
    return
  }

  if (url.pathname === '/' || url.pathname === '/index.html') {
    sendFile(res, path.join(PUBLIC_DIR, 'index.html'))
    return
  }

  if (url.pathname === '/styles.css' || url.pathname === '/app.js') {
    sendFile(res, path.join(PUBLIC_DIR, url.pathname.slice(1)))
    return
  }

  res.writeHead(404)
  res.end('Not found')
})

server.listen(PORT, HOST, () => {
  console.log(`OpenCode ops dashboard running at http://${HOST}:${PORT}`)
})
