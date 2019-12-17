const DEFAULT_PAGE_SIZE = 100;

const SORT_DIRECTION = {
    ASC: 0,
    DESC: 1
};

(() => {
    const table = document.getElementById('datatable');
    if (table) {
        setupTable(table);
    }

    function setupTable(table) {
        const header = table.querySelector('thead');
        const body = table.querySelector('tbody');
        const footer = createPagingRow(table);
        const filter = createFilter(header);
        const data = selectData(body);
        const query = {
            filter: '',
            paging: {
                size: DEFAULT_PAGE_SIZE,
                page: 0,
                pages: Math.ceil(data.length / DEFAULT_PAGE_SIZE)
            },
            sorting: {
                column: 0,
                direction: SORT_DIRECTION.ASC
            }
        };
        const render = () => {
            sortData(data, query.sorting);
            renderData(body, applyQuery(data, query));
            renderPaging(footer, query.paging, page => {
                query.paging.page = page;
                render();
            });
            renderHeader(header, query.sorting);
        };
        filter.addEventListener('input', () => {
            query.filter = filter.value;
            render();
        });
        enhanceHeader(header, column => {
            if (column === query.sorting.column) {
                query.sorting.direction = (query.sorting.direction + 1) % 2;
            } else {
                query.sorting = {
                    column,
                    direction: SORT_DIRECTION.ASC
                };
            }
            render();
        });

        render();
    }

    function createFilter(header) {
        const filter = document.createElement('input');
        filter.classList.add('input');
        filter.placeholder = 'Filter...';
        const filterRow = document.createElement('tr');
        const filterColumn = document.createElement('th');
        filterColumn.colSpan = header.querySelectorAll('th').length;
        filterRow.appendChild(filterColumn);
        filterColumn.appendChild(filter);
        header.appendChild(filterRow);
        return filter;
    }

    function createPagingRow(table) {
        const footer = document.createElement('tfoot');
        const row = document.createElement('tr');
        footer.appendChild(row);
        const column = document.createElement('td');
        column.colSpan = table.querySelectorAll('th').length;
        row.appendChild(column);
        table.appendChild(footer);
        return column;
    }

    function sortData(data, sorting) {
        data.sort((leftRow, rightRow) => {
            const leftField = leftRow.data[sorting.column];
            const rightField = rightRow.data[sorting.column];

            const result = leftField.localeCompare(rightField);
            if (sorting.direction === SORT_DIRECTION.ASC) {
                return result;
            } else {
                return result * -1;
            }
        });
    }

    function enhanceHeader(header, update) {
        const columns = header.querySelectorAll('[data-sortable]');
        for (let i = 0; i < columns.length; i++) {
            const column = columns[i];
            column.classList.add('table__sortable-header');
            column.addEventListener('click', () => update(i));
        }
    }

    function renderHeader(header, sorting) {
        const columns = header.querySelectorAll('.table__sortable-header');
        for (let i = 0; i < columns.length; i++) {
            const column = columns[i];
            column.classList.remove('table__sortable-header--asc', 'table__sortable-header--desc');
            if (i === sorting.column) {
                if (sorting.direction === SORT_DIRECTION.ASC) {
                    column.classList.add('table__sortable-header--asc');
                } else {
                    column.classList.add('table__sortable-header--desc');
                }
            }
        }
    }

    function applyQuery(data, query) {
        const startIndex = query.paging.size * query.paging.page;
        const endIndex = query.paging.size * (query.paging.page + 1);
        const filter = query.filter.toLowerCase().split(' ');
        return data
            .filter(row => filter.every(f => row.data.some(d => d.toLowerCase().includes(f))))
            .filter((row, index) => startIndex <= index && index < endIndex);
    }

    function renderData(tbody, data) {
        clearChildren(tbody);
        for (const row of data) {
            tbody.appendChild(row.node);
        }
    }

    function createPageMoreButton(container) {
        const dotsBtn = createPagerBtn('...');
        dotsBtn.disabled = true;
        container.appendChild(dotsBtn);
    }

    function renderPaging(footer, paging, update) {
        clearChildren(footer);
        if (paging.pages === 1) {
            return;
        }
        const container = document.createElement('div');
        container.classList.add('table-paging');
        const prevBtn = createPagerBtn('<', () => update(paging.page - 1));
        prevBtn.disabled = paging.page === 0;
        container.appendChild(prevBtn);
        if (paging.pages <= 5) {
            createPages(container, paging, update);
        } else if (paging.page <= 3) {
            createPageButton(container, paging, 0, update);
            createPageButton(container, paging, 1, update);
            createPageButton(container, paging, 2, update);
            createPageButton(container, paging, 3, update);
            createPageMoreButton(container);
        } else if (paging.pages - 4 < paging.page) {
            createPageMoreButton(container);
            createPageButton(container, paging, paging.pages - 4, update);
            createPageButton(container, paging, paging.pages - 3, update);
            createPageButton(container, paging, paging.pages - 2, update);
            createPageButton(container, paging, paging.pages - 1, update);
        } else {
            createPageMoreButton(container);
            createPageButton(container, paging, paging.page - 1, update);
            createPageButton(container, paging, paging.page, update);
            createPageButton(container, paging, paging.page + 1, update);
            createPageMoreButton(container);
        }
        const nextBtn = createPagerBtn('>', () => update(paging.page + 1));
        nextBtn.disabled = paging.page === paging.pages - 1;
        container.appendChild(nextBtn);

        footer.appendChild(container);
    }

    function createPages(container, paging, update) {
        for (let i = 0; i < paging.pages; i++) {
            createPageButton(container, paging, i, update);
        }
    }

    function createPageButton(container, paging, page, update) {
        const pageBtn = createPagerBtn(`${page + 1}`, () => update(page));
        if (page === paging.page) {
            pageBtn.classList.add('table-paging__btn--active');
        }
        container.appendChild(pageBtn);
    }

    function createPagerBtn(text, clickListener) {
        const btn = document.createElement('button');
        btn.innerText = text;
        btn.classList.add('table-paging__btn');
        btn.addEventListener('click', clickListener);
        return btn;
    }

    function toColumn(column) {
        return column.textContent.trim();
    }

    function toColumns(row) {
        return {
            node: row,
            data: Array.from(row.querySelectorAll('[data-sortable]')).map(toColumn)
        };
    }

    function selectData(body) {
        return Array.from(body.querySelectorAll('tr')).map(toColumns);
    }

    function clearChildren(root) {
        while (root.firstChild) {
            root.removeChild(root.firstChild);
        }
    }
})();
