@testset "ids: sequential work ids allocate in order" begin
    st = M.State()
    @test M.next_id!(st, :w) == "W-01"
    @test M.next_id!(st, :w) == "W-02"
    M.record_id!(st, "W-09")
    @test M.next_id!(st, :w) == "W-10"
end

@testset "ids: stride and pad width allocate stepped padded ids" begin
    st = M.State()
    st.id_stride = 4
    st.id_offset = 1
    st.id_pad_width = 3
    @test M.next_id!(st, :w) == "W-001"
    @test M.next_id!(st, :w) == "W-005"
end
