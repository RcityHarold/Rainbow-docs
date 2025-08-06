-- 查询特定空间的所有文档，包括私有和已删除的
SELECT 
    id,
    title,
    parent_id,
    is_public,
    is_deleted,
    order_index
FROM document
WHERE space_id = space:80bofplbc9o1pvu3ztot
ORDER BY order_index ASC;

-- 查询只显示公开且未删除的文档（这是发布时会包含的文档）
SELECT 
    id,
    title,
    parent_id,
    is_public,
    is_deleted,
    order_index
FROM document
WHERE space_id = space:80bofplbc9o1pvu3ztot
    AND is_deleted = false
    AND is_public = true
ORDER BY order_index ASC;

-- 查询发布后的文档快照
SELECT 
    id,
    title,
    parent_id,
    original_doc_id,
    publication_id
FROM publication_document
WHERE publication_id = 'space_publication:d4yclfrgw2s5vregxrfl'
ORDER BY order_index ASC;