<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { commands } from "$lib/bindings";
  import { open } from "@tauri-apps/plugin-dialog";

  let parts = $state<any[]>([]);
  let name = $state("");
  let lowBound = $state("");
  let highBound = $state("");
  let imagePath = $state<string | null>(null);
  let editingId = $state<number | null>(null);
  let error = $state("");
  let success = $state("");
  let pickingImage = $state(false);

  onMount(async () => {
    await loadParts();
  });

  async function loadParts() {
    try {
      const result = await commands.getParts();
      if (result.status === "ok") {
        parts = result.data;
      } else {
        error = "فشل في تحميل القطع";
      }
    } catch (e) {
      error = "خطأ في الاتصال";
    }
  }

  async function pickImage() {
    try {
      pickingImage = true;
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [
          { name: "Images", extensions: ["png", "jpg", "jpeg", "webp"] },
        ],
      });

      if (selected) {
        const res = await commands.savePartImage(selected as string);
        if (res.status === "ok") {
          imagePath = res.data;
        } else {
          error = "فشل في حفظ الصورة: " + JSON.stringify(res.error);
        }
      }
    } catch (e) {
      error = "خطأ في اختيار الصورة: " + String(e);
    } finally {
      pickingImage = false;
    }
  }

  async function addPart() {
    if (!name.trim()) {
      error = "الرجاء إدخال اسم القطعة";
      return;
    }

    try {
      const result = await commands.addPart(
        name.trim(),
        lowBound ? parseFloat(lowBound) : null,
        highBound ? parseFloat(highBound) : null,
        imagePath,
      );

      if (result.status === "ok") {
        success = "تمت إضافة القطعة بنجاح";
        resetForm();
        await loadParts();
        setTimeout(() => (success = ""), 3000);
      } else {
        error = "فشل في إضافة القطعة";
      }
    } catch (e) {
      error = "خطأ في الاتصال";
    }
  }

  async function updatePart() {
    if (!name.trim()) {
      error = "الرجاء إدخال اسم القطعة";
      return;
    }

    try {
      const result = await commands.updatePart(
        editingId!,
        name.trim(),
        lowBound ? parseFloat(lowBound) : null,
        highBound ? parseFloat(highBound) : null,
        imagePath,
      );

      if (result.status === "ok") {
        success = "تم تحديث القطعة بنجاح";
        resetForm();
        await loadParts();
        setTimeout(() => (success = ""), 3000);
      } else {
        error = "فشل في تحديث القطعة";
      }
    } catch (e) {
      error = "خطأ في الاتصال";
    }
  }

  function startEdit(part: any) {
    editingId = part.id;
    name = part.name;
    lowBound = part.low_bound?.toString() || "";
    highBound = part.high_bound?.toString() || "";
    imagePath = part.image_path;
  }

  function resetForm() {
    editingId = null;
    name = "";
    lowBound = "";
    highBound = "";
    imagePath = null;
    error = "";
  }

  async function deletePart(id: number) {
    if (!confirm("هل أنت متأكد من حذف هذه القطعة؟")) return;

    try {
      const result = await commands.deletePart(id);
      if (result.status === "ok") {
        success = "تم حذف القطعة";
        await loadParts();
        setTimeout(() => (success = ""), 3000);
      } else {
        error = "فشل في حذف القطعة";
      }
    } catch (e) {
      error = "خطأ في الاتصال";
    }
  }
</script>

<div class="container">
  <div class="header">
    <h1>إدارة القطع</h1>
    <button class="back-btn" onclick={() => goto("/")}>← رجوع</button>
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if success}
    <div class="success">{success}</div>
  {/if}

  <div class="form">
    <h2>{editingId ? "تعديل القطعة" : "إضافة قطعة جديدة"}</h2>

    <div class="form-group">
      <label>اسم القطعة (باللاتينية للفاتورة) *</label>
      <input
        type="text"
        bind:value={name}
        placeholder="Ex: Robinet"
        dir="ltr"
        style="text-align: left;"
      />
      <small style="color: #d16a6c; font-size: 0.8rem; margin-top: 4px; display: block;">
        اكتب الاسم باللاتينية لكي يظهر في PDF.
      </small>
    </div>

    <div class="form-row">
      <div class="form-group">
        <label>الحد الأدنى (اختياري)</label>
        <input type="number" bind:value={lowBound} placeholder="0" step="0.01" />
      </div>

      <div class="form-group">
        <label>الحد الأقصى (اختياري)</label>
        <input type="number" bind:value={highBound} placeholder="0" step="0.01" />
      </div>
    </div>

    <div class="form-group">
      <label>صورة القطعة (اختياري)</label>
      <div class="image-upload-row">
        <button type="button" class="btn-secondary" onclick={pickImage} disabled={pickingImage}>
          {pickingImage ? "جاري الاختيار..." : "اختيار صورة"}
        </button>
        {#if imagePath}
          <span class="image-status">تم اختيار صورة بنجاح ✔</span>
          <button type="button" class="btn-danger-small" onclick={() => (imagePath = null)}>إزالة</button>
        {/if}
      </div>
      <small style="color: #6c6d6c; font-size: 0.8rem; margin-top: 4px; display: block;">
        💡 ملاحظة: يمكنك كتابة أو رسم الاسم العربي على الصورة لكي يراه العميل في الفاتورة.
      </small>
    </div>

    <div class="form-actions">
      {#if editingId}
        <button class="btn-primary" onclick={updatePart}>حفظ التعديلات</button>
        <button class="btn-secondary" onclick={resetForm}>إلغاء</button>
      {:else}
        <button class="btn-primary" onclick={addPart}>إضافة</button>
      {/if}
    </div>
  </div>

  <div class="parts-list">
    <h2>القطع المتاحة ({parts.length})</h2>

    {#if parts.length === 0}
      <div class="empty">لا توجد قطع مسجلة</div>
    {:else}
      <table>
        <thead>
          <tr>
            <th>الصورة</th>
            <th>الاسم</th>
            <th>الحد الأدنى</th>
            <th>الحد الأقصى</th>
            <th>الإجراءات</th>
          </tr>
        </thead>
        <tbody>
          {#each parts as part}
            <tr>
              <td>
                {#if part.image_path}
                  <img
                    src={`asset://localhost/${part.image_path.replace(/\\/g, '/')}`}
                    alt={part.name}
                    class="thumbnail"
                  />
                {:else}
                  <span class="no-image">—</span>
                {/if}
              </td>
              <td dir="auto">{part.name}</td>
              <td>{part.low_bound ?? "—"}</td>
              <td>{part.high_bound ?? "—"}</td>
              <td class="actions">
                <button class="btn-edit" onclick={() => startEdit(part)}>تعديل</button>
                <button class="btn-delete" onclick={() => deletePart(part.id)}>حذف</button>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>

<style>
  /* لا نحتاج :global(body) هنا */

  .container {
    max-width: 900px;
    margin: 0 auto;
    padding: 2rem;
    direction: rtl;
    color: #6c6d6c;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 2rem;
  }

  h1, h2 {
    color: #6c6d6c;
    margin: 0;
  }

  h1 { font-size: 2rem; }
  h2 { font-size: 1.5rem; margin-bottom: 1rem; }

  .back-btn {
    background: #6f3c39;
    color: white;
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 1rem;
    font-weight: 600;
  }

  .back-btn:hover { opacity: 0.9; }

  .error { background: #fee; color: #c33; padding: 1rem; border-radius: 4px; margin-bottom: 1rem; border: 1px solid #fcc; }
  .success { background: #efe; color: #363; padding: 1rem; border-radius: 4px; margin-bottom: 1rem; border: 1px solid #cfc; }

  /* الحاويات: أبيض دافئ #FDFCF8 */
  .form {
    background: #fdfcf8;
    padding: 1.5rem;
    border-radius: 8px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
    margin-bottom: 2rem;
  }

  .form-group { margin-bottom: 1rem; }
  .form-row { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; }

  label {
    display: block;
    color: #6c6d6c;
    margin-bottom: 0.5rem;
    font-weight: 600;
  }

  /* الحقول: كريمي افتراضي، أبيض عند التركيز */
  input {
    width: 100%;
    padding: 0.75rem;
    border: 1px solid #d16a6c;
    border-radius: 4px;
    font-size: 1rem;
    background: #e3e3db !important;
    box-sizing: border-box;
    color: #6c6d6c;
    transition: all 0.2s ease;
  }

  input:focus {
    outline: none;
    border-color: #f14345;
    background: #ffffff !important;
    box-shadow: 0 0 0 3px rgba(241, 67, 69, 0.1);
  }

  .image-upload-row {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .image-status {
    color: #4caf50;
    font-size: 0.9rem;
    font-weight: 600;
  }

  .btn-danger-small {
    background: transparent;
    color: #f14345;
    border: none;
    cursor: pointer;
    font-size: 0.875rem;
    text-decoration: underline;
  }

  .form-actions {
    display: flex;
    gap: 1rem;
    margin-top: 1rem;
  }

  .btn-primary {
    background: #f14345;
    color: white;
    border: none;
    padding: 0.75rem 1.5rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 1rem;
    font-weight: 600;
  }
  .btn-primary:hover { opacity: 0.9; }

  .btn-secondary {
    background: transparent;
    color: #6c6d6c;
    border: 1px solid #6c6d6c;
    padding: 0.75rem 1.5rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 1rem;
    font-weight: 600;
  }
  .btn-secondary:hover { background: #f5f5f5; }

  .parts-list {
    background: #fdfcf8;
    padding: 1.5rem;
    border-radius: 8px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
  }

  .empty { text-align: center; color: #d16a6c; padding: 2rem; }

  table { width: 100%; border-collapse: collapse; margin-top: 1rem; }

  th {
    background: #f4f1ea;
    color: #6c6d6c;
    padding: 0.75rem;
    text-align: right;
    font-weight: 600;
    border-bottom: 2px solid #e3e3db;
  }

  td {
    padding: 0.75rem;
    border-bottom: 1px solid #e3e3db;
    color: #6c6d6c;
  }

  .thumbnail {
    width: 40px;
    height: 40px;
    object-fit: cover;
    border-radius: 4px;
    border: 1px solid #e3e3db;
  }

  .no-image { color: #aaa; }

  .actions { display: flex; gap: 0.5rem; }

  .btn-edit {
    background: #f14345;
    color: white;
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.875rem;
  }
  .btn-edit:hover { opacity: 0.9; }

  .btn-delete {
    background: transparent;
    color: #f14345;
    border: 1px solid #f14345;
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.875rem;
  }
  .btn-delete:hover { background: rgba(241, 67, 69, 0.1); }
</style>
