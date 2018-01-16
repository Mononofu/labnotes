async function reloadIfNewer(current) {
  try {
    let r = await fetch(`/api/version/${current}`);
    let timestamp = await r.text();
    if (timestamp != current) {
      document.location.reload();
    }
  } catch(e) {
  }
  setTimeout(_ => reloadIfNewer(current), 300);
}

async function main(): Promise<void> {
  let r = await fetch('/api/version/unknown');
  let currentTimestamp = await r.text();
  reloadIfNewer(currentTimestamp);
}

main();
