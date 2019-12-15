const table = document.getElementById('datatable');
if (table) {
    //setupTable(table);
}
const toastContainer = document.getElementById('toast-container');

function addQueue(elem) {
    fetch('/api/add', {
        method: 'POST',
        body: JSON.stringify({
            hash: elem.value,
        })
    })
        .then(add_success_snack, add_error_snack);
}

function playNow(elem) {
    fetch('/api/playnow', {
        method: 'POST',
        body: JSON.stringify({
            hash: elem.value,
        })
    })
        .then(play_success_snack, play_error_snack);
}

function next() {
    fetch('/api/next', {
        method: 'POST'
    })
        .then(() => {
            window.location.href = '/queue';
            next_success_snack();
        }, next_error_snack);
}

function clearQueue() {
    fetch('/api/clear', {
        method: 'POST'
    })
        .then(() => {
            window.location.href = '/queue';
            clear_success_snack();
        }, clear_error_snack);
}

function stop() {
    fetch('/api/stop', {
        method: 'POST'
    })
        .then(() => {
            window.location.href = '/queue';
            stop_success_snack();
        }, stop_error_snack);
}

const TYPES = ['info', 'warning', 'success', 'error'];

function add_success_snack() {
    let type = 'success',
        content = 'Added to Queue';

    toast({
        title: content,
        type: type,
        delay: 3000
    });
}

function add_error_snack() {
    let type = 'error',
        content = 'Failed to Add';

    toast({
        title: content,
        type: type,
        delay: 3000
    });
}

function play_success_snack() {
    let type = 'success',
        content = 'Playing Now';

    toast({
        title: content,
        type: type,
        delay: 3000
    });
}

function play_error_snack() {
    let type = 'error',
        content = 'Failed to Play';

    toast({
        title: content,
        type: type,
        delay: 3000
    });
}

function next_success_snack() {
    let type = 'success',
        content = 'Next song playing';

    toast({
        title: content,
        type: type,
        delay: 3000
    });
}

function next_error_snack() {
    let type = 'error',
        content = 'Failed to play next';

    $.toast({
        title: content,
        type: type,
        delay: 3000
    });
}

function clear_success_snack() {
    let type = 'success',
        content = 'Queue cleared';

    toast({
        title: content,
        type: type,
        delay: 3000
    });
}

function clear_error_snack() {
    let type = 'error',
        content = 'Failed to clear queue';

    toast({
        title: content,
        type: type,
        delay: 3000
    });
}

function stop_success_snack() {
    let type = 'success',
        content = 'Player stopped';

    toast({
        title: content,
        type: type,
        delay: 3000
    });
}

function stop_error_snack() {
    let type = 'error',
        content = 'Failed to stop player';

    toast({
        title: content,
        type: type,
        delay: 3000
    });
}

function toast(body) {
    const notification = document.createElement('div');
    notification.classList.add('toast');
    notification.classList.add(`toast-${body.type}`);
    const header = document.createElement('div');
    header.classList.add('toast-header');
    header.innerText = body.title;
    notification.appendChild(header);
    toastContainer.appendChild(notification);

    setTimeout(() => notification.classList.add('toast-show'), 100);

    setTimeout(() => {
        notification.classList.remove('toast-show');
        setTimeout(() => {
            toastContainer.removeChild(notification);
        }, 100);
    }, body.delay);
}